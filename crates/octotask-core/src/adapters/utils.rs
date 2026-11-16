use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};

use super::ssh;
use crate::config::remote_path;
use crate::{Error, Location, Result};

/// Variables that tie git to one repository, as listed by `git rev-parse --local-env-vars`.
/// Inherited from a caller such as a git hook, they would make git ignore the directory it runs in.
const REPO_LOCAL_GIT_VARS: [&str; 15] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
];

/// Where a command runs: the machine, and the directory there.
#[derive(Debug, Clone, Copy)]
pub(super) struct Place<'a> {
    /// `None` is this machine.
    host: Option<&'a str>,
    /// `None` is where the command happens to start: on another machine, the user's home.
    dir: Option<&'a Path>,
}

impl<'a> Place<'a> {
    pub(super) fn here() -> Self {
        Self {
            host: None,
            dir: None,
        }
    }

    pub(super) fn local(dir: &'a Path) -> Self {
        Self {
            host: None,
            dir: Some(dir),
        }
    }

    pub(super) fn host(host: &'a str) -> Self {
        Self {
            host: Some(host),
            dir: None,
        }
    }

    pub(super) fn machine(location: &'a Location) -> Self {
        Self {
            host: location.host(),
            dir: None,
        }
    }

    pub(super) fn in_dir(self, dir: &'a Path) -> Self {
        Self {
            dir: Some(dir),
            ..self
        }
    }

    /// The directory as error messages show it: `host:/path` on another machine.
    fn shown(&self) -> PathBuf {
        match (self.host, self.dir) {
            (None, Some(dir)) => dir.to_path_buf(),
            (None, None) => PathBuf::from("."),
            (Some(host), Some(dir)) => remote_path(host, dir),
            (Some(host), None) => remote_path(host, Path::new("~")),
        }
    }
}

/// Runs `program <args>` and returns its stdout. Any non-zero exit is an error.
pub(super) fn run_checked(program: &'static str, place: Place, args: &[&str]) -> Result<String> {
    let output = run_captured(program, place, args)?;
    if !output.status.success() {
        return Err(failure(program, place, args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Runs a yes/no query such as `test -d`: exit 0 is `true`, exit 1 is `false`,
/// anything else is an error.
pub(super) fn run_query(program: &'static str, place: Place, args: &[&str]) -> Result<bool> {
    let output = run_captured(program, place, args)?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(failure(program, place, args, &output)),
    }
}

/// Runs `program <args>` with stderr left attached to octok's, so the user sees the
/// tool's own progress. A failure carries no stderr: the tool has already printed it.
pub(super) fn run_visible(program: &'static str, place: Place, args: &[&str]) -> Result<()> {
    let status = command(program, place, args)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|source| spawn_error(program, place, source))?;
    succeeded(program, place, args, status)
}

/// Runs `program <args>` attached to octok's terminal, for a tool that asks the user something.
pub(super) fn run_attached(program: &'static str, place: Place, args: &[&str]) -> Result<()> {
    let status = command(program, place, args)
        .stdin(Stdio::inherit())
        .status()
        .map_err(|source| spawn_error(program, place, source))?;
    succeeded(program, place, args, status)
}

fn run_captured(program: &'static str, place: Place, args: &[&str]) -> Result<Output> {
    command(program, place, args)
        .output()
        .map_err(|source| spawn_error(program, place, source))
}

fn succeeded(program: &str, place: Place, args: &[&str], status: ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(failed(program, place, args, status, String::new()))
    }
}

/// Error for a command that ran and failed, carrying its own stderr unchanged.
fn failure(program: &str, place: Place, args: &[&str], output: &Output) -> Error {
    let stderr = String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_string();
    failed(program, place, args, output.status, stderr)
}

fn failed(program: &str, place: Place, args: &[&str], status: ExitStatus, stderr: String) -> Error {
    match place.host {
        Some(host) if status.code() == Some(ssh::OWN_FAILURE) => Error::Ssh {
            host: host.to_string(),
            stderr,
        },
        _ => Error::CommandFailed {
            command: display_command(program, args),
            dir: place.shown(),
            status,
            stderr,
        },
    }
}

fn command(program: &str, place: Place, args: &[&str]) -> Command {
    match place.host {
        Some(host) => ssh::command(host, program, place.dir, args),
        None => local_command(program, place.dir, args),
    }
}

fn local_command(program: &str, dir: Option<&Path>, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        // Fail instead of waiting for a password nobody may be there to type. `gh` runs git too.
        .env("GIT_TERMINAL_PROMPT", "0");
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    for var in REPO_LOCAL_GIT_VARS {
        command.env_remove(var);
    }
    command
}

fn spawn_error(program: &'static str, place: Place, source: io::Error) -> Error {
    let (spawned, dir_is_there) = match place.host {
        Some(_) => (ssh::PROGRAM, true),
        None => (program, place.dir.is_none_or(Path::is_dir)),
    };
    // A missing working directory also fails with `NotFound`.
    if source.kind() == ErrorKind::NotFound && dir_is_there {
        return Error::ToolNotFound(spawned);
    }
    Error::Spawn {
        program: spawned,
        dir: place.shown(),
        source,
    }
}

fn display_command(program: &str, args: &[&str]) -> String {
    let mut words = vec![program.to_string()];
    words.extend(args.iter().map(|arg| redact_credentials(arg)));
    words.join(" ")
}

/// Hides the user info of a url, which can hold a password or a token: `https://***@host/path`.
fn redact_credentials(arg: &str) -> String {
    let Some((scheme, rest)) = arg.split_once("://") else {
        return arg.to_string();
    };
    let authority = rest.split('/').next().unwrap_or_default();
    match authority.rfind('@') {
        Some(at) => format!("{scheme}://***{}", &rest[at..]),
        None => arg.to_string(),
    }
}
