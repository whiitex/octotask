use std::path::PathBuf;
use std::process::ExitStatus;

use crate::utils::CONFIG_VERSION;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid {kind} `{value}`: {reason}")]
    InvalidName {
        kind: &'static str,
        value: String,
        reason: &'static str,
    },
    #[error("invalid git url `{0}`")]
    InvalidUrl(String),
    #[error("cannot derive a repo key from `{0}`: choose one with `--name <key>`")]
    NoRepoKey(String),
    #[error("repo `{0}` is already registered")]
    DuplicateRepo(String),
    #[error("repo `{0}` is not registered")]
    RepoNotFound(String),
    #[error("project `{0}` already exists")]
    ProjectExists(String),
    #[error("project `{0}` not found")]
    ProjectNotFound(String),
    #[error("no project given and no default project set (use `octok project use <name>`)")]
    NoDefaultProject,
    #[error("{path}: unsupported config version {version} (expected {expected})", expected = CONFIG_VERSION)]
    UnsupportedVersion { path: PathBuf, version: u32 },
    #[error("{path}: file declares project `{name}` but is named after `{expected}`")]
    NameMismatch {
        path: PathBuf,
        name: String,
        expected: String,
    },
    #[error("{path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("{path}: cannot serialize: {message}")]
    Serialize { path: PathBuf, message: String },
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot locate home directory (set OCTOTASK_HOME)")]
    NoHome,
    #[error("`{0}` not found: install it or add it to PATH")]
    ToolNotFound(&'static str),
    #[error("cannot run `{program}` in {dir}: {source}")]
    Spawn {
        program: &'static str,
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("`{command}` failed in {dir} ({status}){}", stderr_block(.stderr))]
    CommandFailed {
        command: String,
        dir: PathBuf,
        status: ExitStatus,
        stderr: String,
    },
}

/// The tool's own stderr, on the lines after octok's summary.
fn stderr_block(stderr: &str) -> String {
    if stderr.is_empty() {
        String::new()
    } else {
        format!("\n{stderr}")
    }
}
