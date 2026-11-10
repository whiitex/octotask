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
    #[error("repo `{key}` is already registered ({url})\nchoose another name with `--name <key>`")]
    DuplicateRepo { key: String, url: String },
    #[error("alias `{alias}` is already used by repo `{repo}`")]
    DuplicateAlias { alias: String, repo: String },
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
    #[error("project `{0}` has no repos: add one with `octok repo add <url>`")]
    NoRepos(String),
    #[error("repo `{repo}` is used by workspaces: {}", .workspaces.join(", "))]
    RepoInUse {
        repo: String,
        workspaces: Vec<String>,
    },
    #[error("repo `{repo}` has no branch `{base}` on its remote")]
    BaseNotFound { repo: String, base: String },
    #[error(
        "branch `{branch}` already exists in: {}\nchoose another name, or delete the existing branch first",
        .repos.join(", ")
    )]
    BranchExists { branch: String, repos: Vec<String> },
    #[error("workspace `{name}` already exists at {path}")]
    WorkspaceExists { name: String, path: PathBuf },
    #[error("no workspace for `{0}`: create it with `octok checkout -b {0}`")]
    WorkspaceNotFound(String),
    #[error("not inside a workspace: pass its branch, as in `octok pull <branch>`")]
    NotInWorkspace,
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
