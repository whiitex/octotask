use std::io::{PipeReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use super::utils::{Place, spawn_piped};
use crate::{Location, Result};

/// Runs the commands of a repo in the directory `$1`: the setup `$3` if it is due, then
/// `$2`. It stops them when stdin closes: that is how octok asks for a stop, and what
/// happens by itself when octok dies.
///
/// A stop is SIGTERM to the whole process group, and SIGKILL ten seconds later to what
/// is still there. The group gets the same treatment when the commands end by themselves,
/// so nothing they started outlives them.
///
/// With `$4`, a number of seconds, octok also writes to stdin every few seconds, and that
/// long a silence is a stop too: a connection that died without closing never closes stdin.
///
/// The setup is due until it succeeds, and again when its text changes: the text of the
/// last one that succeeded is kept in the git directory of the checkout.
///
/// The signals go to group `0`, the one of this shell: it must have a group of its own.
const SUPERVISOR: &str = r#"
dir=$1 run=$2 setup=$3 silence=$4
cd "$dir" || exit
exec 3<&0
trap 'stopping=1' TERM

# Returns when octok asks for a stop, or when this shell is signaled.
await_stop() {
    beats=
    [ -z "$silence" ] || beats=$(mktemp) || beats=
    if [ -z "$beats" ]; then
        cat <&3
        return
    fi
    # Each heartbeat makes the file grow, and the end of stdin deletes it.
    { cat <&3 >>"$beats"; rm -f "$beats"; } &
    quiet=0
    seen=
    while [ -z "$signaled" ] && [ "$quiet" -lt "$silence" ]; do
        sleep 1
        now=$(wc -c <"$beats") || break
        if [ "$now" = "$seen" ]; then quiet=$((quiet + 1)); else quiet=0; fi
        seen=$now
    done
    rm -f "$beats"
}

{
    trap 'signaled=1' TERM
    await_stop
    exec 3<&-
    [ -n "$signaled" ] || kill -TERM 0
    sleep 10
    kill -KILL 0
} >/dev/null 2>&1 &
exec 3<&-

# Runs a command to its end and leaves its exit status in `$status`.
# A trapped signal ends `wait` early, with the command still running.
supervised() {
    printf '$ %s\n' "$1"
    sh -c "$1" </dev/null &
    command=$!
    wait "$command"
    status=$?
    while kill -0 "$command" 2>/dev/null; do
        wait "$command"
        status=$?
    done
}

status=0
if [ -n "$setup" ]; then
    marker=$(git rev-parse --git-dir)/octok-setup || exit
    if [ "$(cat "$marker" 2>/dev/null)" != "$(printf '%s' "$setup")" ]; then
        supervised "$setup"
        [ "$status" -ne 0 ] || printf '%s\n' "$setup" >"$marker"
    fi
fi
if [ "$status" -eq 0 ] && [ -z "$stopping" ]; then
    supervised "$run"
fi
[ -n "$stopping" ] || kill -TERM 0
exit "$status"
"#;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);

/// Seconds without a heartbeat after which a command on another machine stops by itself.
const SILENCE_LIMIT: &str = "30";

/// The shell commands of one repo.
pub(crate) struct Commands<'a> {
    pub(crate) run: &'a str,
    /// Run before `run` until it succeeds once, and again when it changes.
    pub(crate) setup: Option<&'a str>,
}

/// Shell commands left running on the machine of a location.
pub(crate) struct Supervised {
    pub(crate) lifeline: Lifeline,
    /// What the commands write to stdout and stderr.
    pub(crate) output: PipeReader,
    pub(crate) process: Child,
}

/// Keeps supervised commands running: dropping it stops them.
pub(crate) struct Lifeline {
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

/// Starts the commands with `sh` in `dir`, on the machine of `location`.
pub(crate) fn supervise(
    location: &Location,
    dir: &Path,
    commands: &Commands,
) -> Result<Supervised> {
    let is_remote = location.host().is_some();
    let dir = dir.to_string_lossy();
    let setup = commands.setup.unwrap_or_default();
    let silence = if is_remote { SILENCE_LIMIT } else { "" };
    let args = ["-c", SUPERVISOR, "sh", &dir, commands.run, setup, silence];
    let (mut process, output) = spawn_piped("sh", Place::machine(location), &args)?;
    let stdin = process.stdin.take().expect("stdin is piped");
    let lifeline = Lifeline {
        stdin: Arc::new(Mutex::new(Some(stdin))),
    };
    if is_remote {
        send_heartbeats(Arc::clone(&lifeline.stdin));
    }
    Ok(Supervised {
        lifeline,
        output,
        process,
    })
}

impl Drop for Lifeline {
    fn drop(&mut self) {
        *self.stdin.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

/// Writes to `stdin` every few seconds, until the lifeline is dropped or nobody reads.
fn send_heartbeats(stdin: Arc<Mutex<Option<ChildStdin>>>) {
    thread::spawn(move || {
        loop {
            thread::sleep(HEARTBEAT_INTERVAL);
            let mut stdin = stdin.lock().unwrap_or_else(PoisonError::into_inner);
            let sent = stdin
                .as_mut()
                .is_some_and(|stdin| stdin.write_all(b"\n").is_ok());
            if !sent {
                break;
            }
        }
    });
}
