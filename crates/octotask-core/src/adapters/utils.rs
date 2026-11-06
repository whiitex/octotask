use std::io::{self, ErrorKind};
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::{Error, Result};

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

/// Runs `program <args>` in `dir` and returns its stdout. Any non-zero exit is an error.
pub(super) fn run_checked(program: &'static str, dir: &Path, args: &[&str]) -> Result<String> {
    let output = run_captured(program, dir, args)?;
    if !output.status.success() {
        return Err(failure(program, dir, args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Runs `program <args>` in `dir` and returns its output whatever the exit status.
pub(super) fn run_captured(program: &'static str, dir: &Path, args: &[&str]) -> Result<Output> {
    command(program, dir, args)
        .output()
        .map_err(|source| spawn_error(program, dir, source))
}

/// Error for a command that ran and failed, carrying its own stderr unchanged.
pub(super) fn failure(program: &'static str, dir: &Path, args: &[&str], output: &Output) -> Error {
    Error::CommandFailed {
        command: display_command(program, args),
        dir: dir.to_path_buf(),
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr)
            .trim_end()
            .to_string(),
    }
}

fn command(program: &str, dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        // Fail instead of waiting for a password nobody may be there to type. `gh` runs git too.
        .env("GIT_TERMINAL_PROMPT", "0");
    for var in REPO_LOCAL_GIT_VARS {
        command.env_remove(var);
    }
    command
}

fn spawn_error(program: &'static str, dir: &Path, source: io::Error) -> Error {
    // A missing working directory also fails with `NotFound`.
    if source.kind() == ErrorKind::NotFound && dir.is_dir() {
        return Error::ToolNotFound(program);
    }
    Error::Spawn {
        program,
        dir: dir.to_path_buf(),
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
