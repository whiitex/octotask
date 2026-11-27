use std::collections::HashMap;
use std::fmt::Display;
use std::io::{self, BufRead, IsTerminal, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitCode, ExitStatus};

use octotask_core::{
    Checkout, CheckoutOutcome, Commit, DevChanges, Error, ErrorKind, FetchOutcome, NewWorkspace,
    Opened, Project, PullOutcome, Registry, RepoResult, Result, Status, UpEvent, WorkspaceStatus,
};
use serde::Serialize;
use serde_json::json;
use signal_hook::consts::SIGKILL;

/// How a location on this machine is written.
pub(crate) const LOCAL: &str = "local";

/// Version of the JSON documents: it changes only when a field is removed or changes meaning.
const SCHEMA: u32 = 1;

/// Exit code of a command that octok cannot read, as clap gives it.
const USAGE: u8 = 2;
/// Exit code of a command that failed for some repos and went well for the others.
const SOME_REPOS_FAILED: u8 = 6;

/// How a command says what it did: text for people, or JSON documents on stdout.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Output {
    json: bool,
}

impl Output {
    pub(crate) fn new(json: bool) -> Self {
        Self { json }
    }

    /// Whether octok may ask a question: only a person at a terminal answers one, and
    /// who asks for JSON is not one.
    pub(crate) fn may_ask(self) -> bool {
        !self.json && io::stdin().is_terminal()
    }

    /// Refuses, with `--json`, a `command` that only works by asking at the terminal.
    pub(crate) fn needs_terminal(self, command: &'static str) -> Result<()> {
        if self.json {
            Err(Error::NeedsTerminal(command))
        } else {
            Ok(())
        }
    }

    /// Says what a command did: `value` as a document with `--json`, else what `text` prints.
    pub(crate) fn result<T: Serialize>(self, value: &T, text: impl FnOnce(&T)) {
        if self.json {
            print_document(value);
        } else {
            text(value);
        }
    }

    /// A line for people, said as soon as a step of a command is done. With `--json`
    /// the document at the end says it all.
    pub(crate) fn note(self, line: impl Display) {
        if !self.json {
            println!("{line}");
        }
    }

    /// Says why a command failed. Returns the exit code of that class of failure.
    pub(crate) fn failure(self, error: &Error) -> ExitCode {
        if self.json {
            print_document(&json!({"error": error}));
        } else {
            eprintln!("error: {error}");
        }
        ExitCode::from(match error.kind() {
            ErrorKind::Other => 1,
            ErrorKind::Usage => USAGE,
            ErrorKind::NotFound => 3,
            ErrorKind::WorkAtRisk => 4,
            ErrorKind::Conflict => 5,
            ErrorKind::Unreachable => 7,
        })
    }
}

/// Says as a document why clap refused the command line. `message` is what clap prints.
pub(crate) fn print_usage_failure(message: &str) -> ExitCode {
    let message = message
        .strip_prefix("error: ")
        .unwrap_or(message)
        .trim_end();
    print_document(&json!({"error": {"code": "usage", "message": message}}));
    ExitCode::from(USAGE)
}

/// Fails if any repo failed.
pub(crate) fn report_exit_code<T>(report: &[RepoResult<T>]) -> ExitCode {
    if report.iter().any(|repo| repo.result.is_err()) {
        ExitCode::from(SOME_REPOS_FAILED)
    } else {
        ExitCode::SUCCESS
    }
}

/// What a command did for each repo of a project, or of one of its workspaces.
#[derive(Serialize)]
pub(crate) struct Report<'a, T> {
    pub(crate) project: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) workspace: Option<&'a str>,
    pub(crate) repos: Vec<RepoResult<T>>,
}

/// A project as its settings leave it.
#[derive(Serialize)]
pub(crate) struct ProjectDocument {
    project: String,
    base: String,
    location: String,
    file: PathBuf,
    /// The repos cloned again where the project moved, when it did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repos: Option<Vec<RepoResult<FetchOutcome>>>,
}

impl ProjectDocument {
    pub(crate) fn read(registry: &Registry, name: &str) -> Result<Self> {
        let project = registry.load_project(name)?;
        let location = project.location.as_ref();
        Ok(Self {
            location: location.map_or_else(|| LOCAL.to_string(), ToString::to_string),
            file: registry.home().project_file(name),
            project: project.name,
            base: project.base,
            repos: None,
        })
    }
}

/// A repo as its settings leave it, in the project or in one `workspace` only.
/// Its commands are never shown: they may hold secrets.
#[derive(Serialize)]
pub(crate) struct RepoDocument<'a> {
    project: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<&'a str>,
    repo: RepoRow<'a>,
}

impl<'a> RepoDocument<'a> {
    pub(crate) fn new(project: &'a Project, workspace: Option<&'a str>, key: &'a str) -> Self {
        Self {
            project: &project.name,
            workspace,
            repo: repo_row(project, key),
        }
    }
}

pub(crate) fn print_projects(out: Output, projects: &[String], default: Option<&str>) {
    let document = json!({"default": default, "projects": projects});
    out.result(&document, |_| {
        if projects.is_empty() {
            println!("no projects yet: create one with `octok project init <name>`");
        }
        for name in projects {
            let marker = if default == Some(name.as_str()) {
                "*"
            } else {
                " "
            };
            println!("{marker} {name}");
        }
    });
}

pub(crate) fn print_repos(out: Output, project: &Project) {
    let rows: Vec<RepoRow> = project
        .repos
        .keys()
        .map(|key| repo_row(project, key))
        .collect();
    let document = json!({"project": project.name, "repos": rows});
    out.result(&document, |_| {
        if rows.is_empty() {
            println!(
                "no repos in {}: add one with `octok repo add <url>`",
                project.name
            );
        } else {
            print_repos_table(&rows);
        }
    });
}

pub(crate) fn print_fetch_report(project: &Project, report: &[RepoResult<FetchOutcome>]) {
    print_report(project, report, |_| "fetched".to_string());
}

pub(crate) fn print_open_report(project: &Project, report: &[RepoResult<Opened>]) {
    print_report(project, report, |opened| format!("opened {}", opened.path));
}

pub(crate) fn print_pull_report(project: &Project, report: &[RepoResult<PullOutcome>]) {
    print_report(project, report, |outcome| {
        pull_message(*outcome).to_string()
    });
}

/// Says what opening the workspace of `branch` did. As text, only the path goes to
/// stdout, so `cd "$(octok branch <name>)"` works.
pub(crate) fn print_workspace(
    out: Output,
    project_name: &str,
    name: &str,
    workspace: &NewWorkspace,
) {
    #[derive(Serialize)]
    struct Document<'a> {
        project: &'a str,
        workspace: &'a str,
        #[serde(flatten)]
        opened: &'a NewWorkspace,
    }
    let document = Document {
        project: project_name,
        workspace: name,
        opened: workspace,
    };
    out.result(&document, |_| {
        print_new_workspace(project_name, name, workspace);
        println!("{}", workspace.path.display());
    });
}

/// Says on stderr what creating or completing a workspace did. Silent when the workspace
/// was already complete.
fn print_new_workspace(project_name: &str, name: &str, workspace: &NewWorkspace) {
    let created = workspace.repos_with(CheckoutOutcome::Created);
    let tracked = workspace.repos_with(CheckoutOutcome::Tracked);
    let restored = workspace.repos_with(CheckoutOutcome::Restored);
    let kept = workspace.repos_with(CheckoutOutcome::Kept);
    if created.is_empty() && tracked.is_empty() && restored.is_empty() {
        return;
    }
    if kept.is_empty() && restored.is_empty() {
        eprintln!("created workspace `{name}` in {project_name}");
    } else {
        eprintln!("completed workspace `{name}` in {project_name}");
    }
    if !restored.is_empty() {
        eprintln!("  branch restored from the cache: {}", restored.join(", "));
    }
    if !tracked.is_empty() {
        eprintln!("  branch from the remote: {}", tracked.join(", "));
    }
    let only_new_branches = kept.is_empty() && tracked.is_empty() && restored.is_empty();
    if !created.is_empty() && !only_new_branches {
        eprintln!("  new branch from the base: {}", created.join(", "));
    }
    for repo in workspace
        .repos
        .iter()
        .filter(|repo| repo.on_another_machine)
    {
        eprintln!("  {}: {}", repo.key, repo.path.display());
    }
}

/// Prints one line per repo on stdout and the errors on stderr.
fn print_report<T>(project: &Project, report: &[RepoResult<T>], describe: impl Fn(&T) -> String) {
    let labels: Vec<String> = report
        .iter()
        .map(|repo| project_repo_label(project, &repo.key))
        .collect();
    let label_width = max_width(labels.iter().map(String::as_str));
    for (repo, label) in report.iter().zip(labels) {
        match &repo.result {
            Ok(outcome) => println!("{label:label_width$}  {}", describe(outcome)),
            Err(error) => {
                println!("{label:label_width$}  failed");
                eprintln!("error: {label}: {error}");
            }
        }
    }
}

/// Asks a yes/no question and reads one line. Anything but `y` or `yes` means no.
pub(crate) fn confirm(question: &str, mut input: impl BufRead, mut output: impl Write) -> bool {
    let mut answer = String::new();
    let asked = write!(output, "{question} [y/N] ").and_then(|()| output.flush());
    if asked.is_err() || input.read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// Asks for a value, showing its default, and reads one line.
/// `None` when the input has ended, an empty text when the answer is just Enter.
pub(crate) fn ask(
    question: &str,
    default: &str,
    mut input: impl BufRead,
    mut output: impl Write,
) -> Option<String> {
    let asked = if default.is_empty() {
        write!(output, "{question}: ")
    } else {
        write!(output, "{question} [{default}]: ")
    };
    asked.and_then(|()| output.flush()).ok()?;
    let mut answer = String::new();
    match input.read_line(&mut answer) {
        Ok(read) if read > 0 => Some(answer.trim().to_string()),
        _ => None,
    }
}

/// What `octok up` puts in front of the lines of each repo, by repo key: the alias or
/// the key, padded to one width.
pub(crate) fn up_labels<'a>(
    project: &Project,
    repos: impl Iterator<Item = &'a str>,
) -> HashMap<String, String> {
    let names: Vec<(&str, &str)> = repos
        .map(|key| (key, project.repos[key].alias.as_deref().unwrap_or(key)))
        .collect();
    let width = max_width(names.iter().map(|(_key, name)| *name));
    names
        .into_iter()
        .map(|(key, name)| (key.to_string(), format!("{name:<width$}")))
        .collect()
}

/// The output of the repos goes to stdout, and what octok says about them to stderr.
/// `stopping` says that the user has asked for a stop.
pub(crate) fn print_up_event(labels: &HashMap<String, String>, event: UpEvent, stopping: bool) {
    match event {
        UpEvent::Output { repo, line } => println!("{} | {line}", labels[repo]),
        UpEvent::Exited { repo, status } => {
            eprintln!("{} | {}", labels[repo], how_it_ended(status, stopping));
        }
    }
}

fn how_it_ended(status: ExitStatus, stopping: bool) -> String {
    match (stopping, status.signal()) {
        (true, Some(SIGKILL)) => "killed: it was still running 10 seconds after the stop".into(),
        (true, _) => "stopped".into(),
        (false, _) => format!("ended ({status})"),
    }
}

/// Lists what is saved of the dev changes of each repo, as text with the diff of the
/// tracked files when `with_diff`. The files git does not track are only named, in the
/// document too: they hold secrets.
pub(crate) fn print_dev_changes(
    out: Output,
    project: &Project,
    repos: &[DevChanges],
    with_diff: bool,
) {
    let rows: Vec<_> = repos
        .iter()
        .map(|changes| {
            json!({
                "key": changes.repo,
                "alias": repo_alias(project, &changes.repo),
                "changed": changes.changed,
                "new": changes.new,
                "diff": changes.diff,
            })
        })
        .collect();
    let document = json!({"project": project.name, "repos": rows});
    out.result(&document, |_| {
        print_dev_changes_text(project, repos, with_diff);
    });
}

fn print_dev_changes_text(project: &Project, repos: &[DevChanges], with_diff: bool) {
    for changes in repos {
        println!("{}", project_repo_label(project, &changes.repo));
        if changes.changed.is_empty() && changes.new.is_empty() {
            println!("  no dev changes");
        }
        for path in &changes.changed {
            println!("  changed  {path}");
        }
        for path in &changes.new {
            println!("  new      {path}");
        }
        if with_diff && !changes.diff.is_empty() {
            println!("\n{}\n", changes.diff);
        }
    }
}

/// How a repo is shown: its key, followed by the alias when it has one.
pub(crate) fn repo_label(key: &str, alias: Option<&str>) -> String {
    match alias {
        Some(alias) => format!("{key} ({alias})"),
        None => key.to_string(),
    }
}

pub(crate) fn print_status(status: &Status) {
    println!("project: {}\n", status.project);
    print_status_repos(status);
    println!();
    print_status_workspaces(status);
    print_orphan_caches(status);
}

/// One JSON document on one line of stdout: `value` with the version of the documents.
fn print_document<T: Serialize>(value: &T) {
    #[derive(Serialize)]
    struct Document<'a, T> {
        schema: u32,
        #[serde(flatten)]
        body: &'a T,
    }
    let document = Document {
        schema: SCHEMA,
        body: value,
    };
    let line = serde_json::to_string(&document).expect("a document is plain data");
    println!("{line}");
}

#[derive(Serialize)]
struct RepoRow<'a> {
    key: &'a str,
    alias: Option<&'a str>,
    url: &'a str,
    /// The base branch in effect: the repo's own, else the project's.
    base: &'a str,
    location: String,
}

fn repo_row<'a>(project: &'a Project, key: &'a str) -> RepoRow<'a> {
    let repo = &project.repos[key];
    RepoRow {
        key,
        alias: repo.alias.as_deref(),
        url: &repo.url,
        base: project.effective_base(repo),
        location: project.effective_location(repo, None).to_string(),
    }
}

fn print_repos_table(rows: &[RepoRow]) {
    let labels: Vec<String> = rows
        .iter()
        .map(|row| repo_label(row.key, row.alias))
        .collect();
    let label_width = max_width(labels.iter().map(String::as_str));
    let base_width = max_width(rows.iter().map(|row| row.base));
    // A project that lives on this machine only keeps the table it always had.
    let all_local = rows.iter().all(|row| row.location == LOCAL);
    let url_width = max_width(rows.iter().map(|row| row.url));
    for (row, label) in rows.iter().zip(labels) {
        if all_local {
            println!(
                "{label:label_width$}  {:base_width$}  {}",
                row.base, row.url
            );
        } else {
            println!(
                "{label:label_width$}  {:base_width$}  {:url_width$}  {}",
                row.base, row.url, row.location
            );
        }
    }
}

fn pull_message(outcome: PullOutcome) -> &'static str {
    match outcome {
        PullOutcome::Updated => "updated",
        PullOutcome::UpToDate => "up to date",
        PullOutcome::NoRemoteBranch => "skipped: branch is not on the remote yet",
        PullOutcome::Diverged => "skipped: diverged from the remote branch",
        PullOutcome::Detached => "skipped: detached HEAD",
        PullOutcome::NoWorktree => "skipped: no worktree in this workspace",
    }
}

fn print_status_repos(status: &Status) {
    if status.repos.is_empty() {
        println!("repos: none (add one with `octok repo add <url>`)");
        return;
    }
    println!("repos:");
    let labels: Vec<String> = status
        .repos
        .iter()
        .map(|repo| repo_label(&repo.key, repo.alias.as_deref()))
        .collect();
    let label_width = max_width(labels.iter().map(String::as_str));
    let base_width = max_width(status.repos.iter().map(|repo| repo.base.as_str()));
    let sha_width = sha_width(status.repos.iter().filter_map(|repo| repo.commit.as_ref()));
    for (repo, label) in status.repos.iter().zip(labels) {
        let commit = match &repo.commit {
            Some(commit) => commit_line(commit, sha_width),
            None if repo.cached => format!("branch `{}` is not on the remote", repo.base),
            None => "not fetched (run `octok fetch`)".to_string(),
        };
        let place = if repo.location == LOCAL {
            String::new()
        } else {
            format!("  [{}]", repo.location)
        };
        println!(
            "  {label:label_width$}  {:base_width$}  {commit}{place}",
            repo.base
        );
    }
}

fn print_status_workspaces(status: &Status) {
    if status.workspaces.is_empty() {
        println!("workspaces: none (create one with `octok branch <name>`)");
        return;
    }
    println!("workspaces:");
    for workspace in &status.workspaces {
        println!("  {}  {}", workspace.name, workspace.path.display());
        print_checkouts(status, workspace);
    }
}

/// One row per repo of the project, so a repo without a worktree stands out.
fn print_checkouts(status: &Status, workspace: &WorkspaceStatus) {
    let checkout_of = |key: &str| workspace.checkouts.iter().find(|c| c.repo == key);
    let readable = || workspace.checkouts.iter().filter(|c| c.commit.is_some());
    let labels: Vec<String> = status
        .repos
        .iter()
        .map(|repo| repo_label(&repo.key, repo.alias.as_deref()))
        .collect();
    let label_width = max_width(labels.iter().map(String::as_str));
    let branch_width = max_width(readable().map(branch_label));
    let sha_width = sha_width(readable().filter_map(|c| c.commit.as_ref()));
    let workspace_branch = workspace.branch.as_deref().unwrap_or("<branch>");
    for (repo, label) in status.repos.iter().zip(labels) {
        let detail = match checkout_of(&repo.key) {
            None => format!("no worktree: add it with `octok checkout {workspace_branch}`"),
            Some(checkout) => match &checkout.commit {
                Some(commit) => format!(
                    "{:branch_width$}  {}{}{}",
                    branch_label(checkout),
                    commit_line(commit, sha_width),
                    unpublished(checkout),
                    place_elsewhere(workspace, checkout)
                ),
                None => format!(
                    "cannot be read: {}",
                    last_line(
                        &checkout
                            .error
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()
                    )
                ),
            },
        };
        println!("    {label:label_width$}  {detail}");
    }
}

/// The path of a worktree that is not under the folder shown for its workspace:
/// it lives on another machine.
fn place_elsewhere(workspace: &WorkspaceStatus, checkout: &Checkout) -> String {
    if checkout.path.starts_with(&workspace.path) {
        String::new()
    } else {
        format!("  [{}]", checkout.path.display())
    }
}

/// Marks a branch that the remote does not have: a new one, not pushed yet.
fn unpublished(checkout: &Checkout) -> &'static str {
    if checkout.branch.is_some() && !checkout.on_remote {
        "  [not on the remote]"
    } else {
        ""
    }
}

fn branch_label(checkout: &Checkout) -> &str {
    checkout.branch.as_deref().unwrap_or("(detached)")
}

fn print_orphan_caches(status: &Status) {
    if status.orphan_caches.is_empty() {
        return;
    }
    println!("\norphan caches (not in the project file):");
    for path in &status.orphan_caches {
        println!("  {}", path.display());
    }
}

fn project_repo_label(project: &Project, key: &str) -> String {
    repo_label(key, repo_alias(project, key))
}

fn repo_alias<'a>(project: &'a Project, key: &str) -> Option<&'a str> {
    project
        .repos
        .get(key)
        .and_then(|repo| repo.alias.as_deref())
}

/// The tool's own message: octok's summary of the failed command comes before it.
fn last_line(error: &str) -> &str {
    error.lines().last().unwrap_or_default()
}

/// git abbreviates hashes to different lengths depending on the repo.
fn commit_line(commit: &Commit, sha_width: usize) -> String {
    format!(
        "{:sha_width$}  {} ({})",
        commit.sha, commit.subject, commit.age
    )
}

fn sha_width<'a>(commits: impl Iterator<Item = &'a Commit>) -> usize {
    max_width(commits.map(|commit| commit.sha.as_str()))
}

pub(crate) fn max_width<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    values.map(str::len).max().unwrap_or(0)
}
