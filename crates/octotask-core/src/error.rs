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
    #[error("invalid location `{value}`: {reason}")]
    InvalidLocation { value: String, reason: &'static str },
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
        "branch `{branch}` already exists on the remote of: {}\ncheck it out with `octok checkout {branch}`, or choose another name",
        .repos.join(", ")
    )]
    BranchExists { branch: String, repos: Vec<String> },
    #[error("no repo has a branch `{0}` on its remote: create it with `octok checkout -b {0}`")]
    BranchNotFound(String),
    #[error("repo `{repo}` has no branch `{branch}` on its remote")]
    BranchNotOnRemote { repo: String, branch: String },
    #[error(
        "branch `{branch}` of `{repo}` is already checked out in {path}\na branch lives in one workspace at a time: use that workspace, or remove it first"
    )]
    BranchInUse {
        repo: String,
        branch: String,
        path: PathBuf,
    },
    #[error(
        "nothing to review: choose a pull request or a branch for at least one repo, with `--pr <repo>=<number>` or `--branch <repo>=<branch>`, or by answering `octok pr review` at a terminal"
    )]
    NothingToReview,
    #[error("repo `{0}` is given more than once: choose one pull request or one branch for it")]
    RepoChosenTwice(String),
    #[error(
        "pull request #{number} of `{repo}` comes from a fork: octok can only review a branch of the repo itself"
    )]
    ForkPullRequest { repo: String, number: u64 },
    #[error(
        "workspace `{name}` has work that would be lost:\n{}\nremove it anyway with `octok rm --force <branch>`",
        indented(.reasons)
    )]
    WorkspaceHasWork { name: String, reasons: Vec<String> },
    #[error(
        "project `{project}` still has workspaces: {}\nremove them first with `octok rm <branch>`",
        .workspaces.join(", ")
    )]
    ProjectInUse {
        project: String,
        workspaces: Vec<String>,
    },
    #[error(
        "workspace `{name}` is the one of the branch `{branch}`, whose name gives the same folder\nremove it with `octok rm {branch}`, or use another branch name"
    )]
    WorkspaceOfAnotherBranch { name: String, branch: String },
    #[error(
        "workspace `{name}` already reviews other branches: {}\nremove it with `octok rm {name}`, or choose another name with `--name`",
        .reviewed.join(", ")
    )]
    ReviewsOtherBranches { name: String, reviewed: Vec<String> },
    #[error("workspace `{name}` already exists at {path}")]
    WorkspaceExists { name: String, path: PathBuf },
    #[error(
        "no workspace for `{0}`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch"
    )]
    WorkspaceNotFound(String),
    #[error("not inside a workspace: pass its branch, as in `{0}`")]
    NotInWorkspace(&'static str),
    #[error(
        "no repo of workspace `{0}` has a `run` command: set one with `octok repo set <repo> --run <command>`"
    )]
    NothingToRun(String),
    #[error("repo `{0}` has no `run` command: set one with `octok repo set {0} --run <command>`")]
    NoRunCommand(String),
    #[error(
        "repo `{repo}` has no worktree in workspace `{workspace}`: add it with `octok checkout <branch>`"
    )]
    NoWorktree { repo: String, workspace: String },
    #[error(
        "the dev changes of `{repo}` no longer apply to: {}\nThe branch changed the same lines, or lacks those files. `--allow-stale-dev` creates the workspace with those files as they are in git: make the dev changes again there, then run `octok dev save`.",
        .files.join(", ")
    )]
    StaleDevChanges { repo: String, files: Vec<String> },
    #[error("no checkout has the file `{0}`")]
    DevFileNotFound(String),
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
    #[error(
        "cannot run commands on `{host}` over ssh{}\n\
         check that `ssh {host}` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`\n\
         if its key has a passphrase, run `octok set-ssh-passphrase {host}`",
        stderr_block(.stderr)
    )]
    Ssh { host: String, stderr: String },
    #[error("`{command}` printed `{output}`, which octok does not understand")]
    UnexpectedOutput { command: String, output: String },
    #[error("`{command}` failed in {dir} ({status}){}", stderr_block(.stderr))]
    CommandFailed {
        command: String,
        dir: PathBuf,
        status: ExitStatus,
        stderr: String,
    },
}

fn indented(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The tool's own stderr, on the lines after octok's summary.
fn stderr_block(stderr: &str) -> String {
    if stderr.is_empty() {
        String::new()
    } else {
        format!("\n{stderr}")
    }
}
