use std::path::PathBuf;
use std::process::ExitStatus;

use serde::ser::{Serialize, SerializeMap, Serializer};
use serde_json::{Value, json};

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
    #[error("`{0}` asks at the terminal, which `--json` never does: run it without `--json`")]
    NeedsTerminal(&'static str),
}

/// What went wrong, in the few classes a caller may act on without reading the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The request is not one octok can take: a name it refuses, a project left out.
    Usage,
    NotFound,
    /// Going on would lose work that exists nowhere else.
    WorkAtRisk,
    /// The request conflicts with what exists: already there, in use, no longer applies.
    Conflict,
    /// Another machine or a program octok needs cannot be reached.
    Unreachable,
    Other,
}

impl Error {
    /// A name for this error that never changes, for programs. The message is for people.
    pub fn code(&self) -> &'static str {
        self.code_and_details().0
    }

    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::InvalidName { .. }
            | Self::InvalidLocation { .. }
            | Self::InvalidUrl(_)
            | Self::NoRepoKey(_)
            | Self::NoDefaultProject
            | Self::NothingToReview
            | Self::RepoChosenTwice(_)
            | Self::NotInWorkspace(_)
            | Self::NeedsTerminal(_) => ErrorKind::Usage,
            Self::RepoNotFound(_)
            | Self::ProjectNotFound(_)
            | Self::NoRepos(_)
            | Self::BaseNotFound { .. }
            | Self::BranchNotFound(_)
            | Self::BranchNotOnRemote { .. }
            | Self::WorkspaceNotFound(_)
            | Self::NothingToRun(_)
            | Self::NoRunCommand(_)
            | Self::NoWorktree { .. }
            | Self::DevFileNotFound(_) => ErrorKind::NotFound,
            Self::WorkspaceHasWork { .. } => ErrorKind::WorkAtRisk,
            Self::DuplicateRepo { .. }
            | Self::DuplicateAlias { .. }
            | Self::ProjectExists(_)
            | Self::RepoInUse { .. }
            | Self::BranchExists { .. }
            | Self::BranchInUse { .. }
            | Self::ProjectInUse { .. }
            | Self::WorkspaceOfAnotherBranch { .. }
            | Self::ReviewsOtherBranches { .. }
            | Self::WorkspaceExists { .. }
            | Self::StaleDevChanges { .. } => ErrorKind::Conflict,
            Self::ToolNotFound(_) | Self::Spawn { .. } | Self::Ssh { .. } => ErrorKind::Unreachable,
            Self::UnsupportedVersion { .. }
            | Self::NameMismatch { .. }
            | Self::Parse { .. }
            | Self::Serialize { .. }
            | Self::Io { .. }
            | Self::ForkPullRequest { .. }
            | Self::NoHome
            | Self::UnexpectedOutput { .. }
            | Self::CommandFailed { .. } => ErrorKind::Other,
        }
    }

    /// The code of the error and the data it carries, by name. Both are an interface: a
    /// released code or name keeps its meaning, whatever the variant is called later.
    fn code_and_details(&self) -> (&'static str, Value) {
        match self {
            Self::InvalidName {
                kind,
                value,
                reason,
            } => (
                "invalid_name",
                json!({"kind": kind, "value": value, "reason": reason}),
            ),
            Self::InvalidLocation { value, reason } => (
                "invalid_location",
                json!({"value": value, "reason": reason}),
            ),
            Self::InvalidUrl(url) => ("invalid_url", json!({"url": url})),
            Self::NoRepoKey(url) => ("no_repo_key", json!({"url": url})),
            Self::DuplicateRepo { key, url } => {
                ("duplicate_repo", json!({"repo": key, "url": url}))
            }
            Self::DuplicateAlias { alias, repo } => {
                ("duplicate_alias", json!({"alias": alias, "repo": repo}))
            }
            Self::RepoNotFound(repo) => ("repo_not_found", json!({"repo": repo})),
            Self::ProjectExists(project) => ("project_exists", json!({"project": project})),
            Self::ProjectNotFound(project) => ("project_not_found", json!({"project": project})),
            Self::NoDefaultProject => ("no_default_project", json!({})),
            Self::UnsupportedVersion { path, version } => (
                "unsupported_version",
                json!({"path": path, "version": version, "expected": CONFIG_VERSION}),
            ),
            Self::NameMismatch {
                path,
                name,
                expected,
            } => (
                "name_mismatch",
                json!({"path": path, "name": name, "expected": expected}),
            ),
            Self::Parse { path, .. } => ("parse", json!({"path": path})),
            Self::Serialize { path, .. } => ("serialize", json!({"path": path})),
            Self::Io { path, .. } => ("io", json!({"path": path})),
            Self::NoRepos(project) => ("no_repos", json!({"project": project})),
            Self::RepoInUse { repo, workspaces } => (
                "repo_in_use",
                json!({"repo": repo, "workspaces": workspaces}),
            ),
            Self::BaseNotFound { repo, base } => {
                ("base_not_found", json!({"repo": repo, "base": base}))
            }
            Self::BranchExists { branch, repos } => {
                ("branch_exists", json!({"branch": branch, "repos": repos}))
            }
            Self::BranchNotFound(branch) => ("branch_not_found", json!({"branch": branch})),
            Self::BranchNotOnRemote { repo, branch } => (
                "branch_not_on_remote",
                json!({"repo": repo, "branch": branch}),
            ),
            Self::BranchInUse { repo, branch, path } => (
                "branch_in_use",
                json!({"repo": repo, "branch": branch, "path": path}),
            ),
            Self::NothingToReview => ("nothing_to_review", json!({})),
            Self::RepoChosenTwice(repo) => ("repo_chosen_twice", json!({"repo": repo})),
            Self::ForkPullRequest { repo, number } => {
                ("fork_pull_request", json!({"repo": repo, "number": number}))
            }
            Self::WorkspaceHasWork { name, reasons } => (
                "workspace_has_work",
                json!({"workspace": name, "reasons": reasons}),
            ),
            Self::ProjectInUse {
                project,
                workspaces,
            } => (
                "project_in_use",
                json!({"project": project, "workspaces": workspaces}),
            ),
            Self::WorkspaceOfAnotherBranch { name, branch } => (
                "workspace_of_another_branch",
                json!({"workspace": name, "branch": branch}),
            ),
            Self::ReviewsOtherBranches { name, reviewed } => (
                "reviews_other_branches",
                json!({"workspace": name, "reviewed": reviewed}),
            ),
            Self::WorkspaceExists { name, path } => {
                ("workspace_exists", json!({"workspace": name, "path": path}))
            }
            Self::WorkspaceNotFound(workspace) => {
                ("workspace_not_found", json!({"workspace": workspace}))
            }
            Self::NotInWorkspace(usage) => ("not_in_workspace", json!({"usage": usage})),
            Self::NothingToRun(workspace) => ("nothing_to_run", json!({"workspace": workspace})),
            Self::NoRunCommand(repo) => ("no_run_command", json!({"repo": repo})),
            Self::NoWorktree { repo, workspace } => {
                ("no_worktree", json!({"repo": repo, "workspace": workspace}))
            }
            Self::StaleDevChanges { repo, files } => {
                ("stale_dev_changes", json!({"repo": repo, "files": files}))
            }
            Self::DevFileNotFound(file) => ("dev_file_not_found", json!({"file": file})),
            Self::NoHome => ("no_home", json!({})),
            Self::ToolNotFound(tool) => ("tool_not_found", json!({"tool": tool})),
            Self::Spawn { program, dir, .. } => ("spawn", json!({"program": program, "dir": dir})),
            Self::Ssh { host, stderr } => ("ssh", json!({"host": host, "stderr": stderr})),
            Self::UnexpectedOutput { command, output } => (
                "unexpected_output",
                json!({"command": command, "output": output}),
            ),
            Self::CommandFailed {
                command,
                dir,
                status,
                stderr,
            } => (
                "command_failed",
                json!({
                    "command": command,
                    "dir": dir,
                    "exit_code": status.code(),
                    "stderr": stderr,
                }),
            ),
            Self::NeedsTerminal(command) => ("needs_terminal", json!({"command": command})),
        }
    }
}

/// `{"code", "message"}`, followed by the data the error carries.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let (code, details) = self.code_and_details();
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("code", code)?;
        map.serialize_entry("message", &self.to_string())?;
        for (name, value) in details.as_object().into_iter().flatten() {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
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
