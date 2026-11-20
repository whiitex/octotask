//! One `sh` for a whole page, so `cd`, variables and files carry from one command to the next.

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

/// Printed after each command, to tell where its output ends.
const DONE: &str = "\u{1e}octok-book-done\u{1e}";

/// Long enough for any example, short enough that a command waiting forever fails the test.
const TIMEOUT: Duration = Duration::from_secs(120);

pub struct Shell {
    child: Child,
    stdin: ChildStdin,
    output: Receiver<Vec<u8>>,
    pending: Vec<u8>,
}

impl Shell {
    /// Starts `sh` with the environment and folder of `command`, which must run `sh`.
    pub fn start(mut command: Command) -> Self {
        let (reader, writer) = std::io::pipe().unwrap();
        command
            .stdin(Stdio::piped())
            .stdout(writer.try_clone().unwrap())
            .stderr(writer);
        let mut child = command.spawn().unwrap();
        // The pipe ends only when `sh` and everything it started have closed it.
        drop(command);
        let stdin = child.stdin.take().unwrap();
        Self {
            child,
            stdin,
            output: read_in_background(reader),
            pending: Vec::new(),
        }
    }

    /// Runs one command line and returns what it printed on stdout and stderr, interleaved.
    /// The command reads nothing: its stdin is empty.
    pub fn run(&mut self, command: &str) -> Result<String, String> {
        // The braces keep a `cd` in this shell, and the newline ends a trailing `# comment`.
        let script = format!("{{ {command}\n}} </dev/null\nprintf '%s' '{DONE}'\n");
        self.stdin
            .write_all(script.as_bytes())
            .map_err(|error| format!("the shell has ended: {error}"))?;
        loop {
            if let Some(end) = find(&self.pending, DONE.as_bytes()) {
                let output: Vec<u8> = self.pending.drain(..end + DONE.len()).collect();
                return Ok(String::from_utf8_lossy(&output[..end]).into_owned());
            }
            match self.output.recv_timeout(TIMEOUT) {
                Ok(chunk) => self.pending.extend(chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(format!("still running after {}s", TIMEOUT.as_secs()));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(format!(
                        "the shell has ended:\n{}",
                        String::from_utf8_lossy(&self.pending)
                    ));
                }
            }
        }
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_in_background(mut reader: impl Read + Send + 'static) -> Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0; 8192];
        while let Ok(read @ 1..) = reader.read(&mut buffer) {
            if sender.send(buffer[..read].to_vec()).is_err() {
                break;
            }
        }
    });
    receiver
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
