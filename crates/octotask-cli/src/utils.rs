use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitCode, ExitStatus};

use octotask_core::{
    Checkout, Commit, DevChanges, Project, PullOutcome, Result, Status, UpEvent, WorkspaceStatus,
};
use serde_json::json;
use signal_hook::consts::SIGKILL;

/// How a location on this machine is written.
pub(crate) const LOCAL: &str = "local";

pub(crate) fn print_projects(projects: &[String], default: Option<&str>, json: bool) {
    if json {
        println!(
            "{}",
            json!({"schema": 1, "default": default, "projects": projects})
        );
    } else if projects.is_empty() {
        println!("no projects yet: create one with `octok project init <name>`");
    } else {
        for name in projects {
            let marker = if default == Some(name.as_str()) {
                "*"
            } else {
                " "
            };
            println!("{marker} {name}");
        }
    }
}

pub(crate) fn print_repos(project: &Project, json: bool) {
    let rows: Vec<RepoRow> = project
        .repos
        .iter()
        .map(|(key, repo)| RepoRow {
            key,
            alias: repo.alias.as_deref(),
            url: &repo.url,
            base: project.effective_base(repo),
            location: project.effective_location(repo, None).to_string(),
        })
        .collect();
    if json {
        print_repos_json(&project.name, &rows);
    } else if rows.is_empty() {
        println!(
            "no repos in {}: add one with `octok repo add <url>`",
            project.name
        );
    } else {
        print_repos_table(&rows);
    }
}

pub(crate) fn print_pull_report(
    project: &Project,
    report: Vec<(String, Result<PullOutcome>)>,
) -> ExitCode {
    print_report(project, report, |outcome| pull_message(outcome).to_string())
}

/// Prints one line per repo on stdout and the errors on stderr. Fails if any repo failed.
pub(crate) fn print_report<T>(
    project: &Project,
    report: Vec<(String, Result<T>)>,
    describe: impl Fn(T) -> String,
) -> ExitCode {
    let rows: Vec<(String, Result<T>)> = report
        .into_iter()
        .map(|(key, result)| (project_repo_label(project, &key), result))
        .collect();
    let label_width = max_width(rows.iter().map(|(label, _)| label.as_str()));
    let mut code = ExitCode::SUCCESS;
    for (label, result) in rows {
        match result {
            Ok(outcome) => println!("{label:label_width$}  {}", describe(outcome)),
            Err(error) => {
                println!("{label:label_width$}  failed");
                eprintln!("error: {label}: {error}");
                code = ExitCode::FAILURE;
            }
        }
    }
    code
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

/// Lists what is saved of the dev changes of each repo, with the diff of the tracked
/// files when `with_diff`. The files git does not track are only named: they hold secrets.
pub(crate) fn print_dev_changes(project: &Project, repos: &[DevChanges], with_diff: bool) {
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

/// Like the text of `print_dev_changes` with the diff: the files git does not track are
/// only named here too.
pub(crate) fn print_dev_changes_json(project: &Project, repos: &[DevChanges]) {
    let repos: Vec<_> = repos
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
    println!(
        "{}",
        json!({"schema": 1, "project": project.name, "repos": repos})
    );
}

/// How a repo is shown: its key, followed by the alias when it has one.
pub(crate) fn repo_label(key: &str, alias: Option<&str>) -> String {
    match alias {
        Some(alias) => format!("{key} ({alias})"),
        None => key.to_string(),
    }
}

pub(crate) fn print_status(status: &Status, json: bool) {
    if json {
        let mut value = serde_json::to_value(status).expect("status is plain data");
        value["schema"] = json!(1);
        println!("{value}");
    } else {
        println!("project: {}\n", status.project);
        print_status_repos(status);
        println!();
        print_status_workspaces(status);
        print_orphan_caches(status);
    }
}

struct RepoRow<'a> {
    key: &'a str,
    alias: Option<&'a str>,
    url: &'a str,
    base: &'a str,
    location: String,
}

fn print_repos_json(project_name: &str, rows: &[RepoRow]) {
    let repos: Vec<_> = rows
        .iter()
        .map(|row| {
            json!({
                "key": row.key,
                "alias": row.alias,
                "url": row.url,
                "base": row.base,
                "location": row.location,
            })
        })
        .collect();
    println!(
        "{}",
        json!({"schema": 1, "project": project_name, "repos": repos})
    );
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
    let workspace_branch = workspace
        .checkouts
        .iter()
        .find_map(|c| c.branch.as_deref())
        .unwrap_or("<branch>");
    for (repo, label) in status.repos.iter().zip(labels) {
        let detail = match checkout_of(&repo.key) {
            None => format!("no worktree: add it with `octok branch {workspace_branch}`"),
            Some(checkout) => match &checkout.commit {
                Some(commit) => format!(
                    "{:branch_width$}  {}{}",
                    branch_label(checkout),
                    commit_line(commit, sha_width),
                    place_elsewhere(workspace, checkout)
                ),
                None => format!(
                    "cannot be read: {}",
                    last_line(checkout.error.as_deref().unwrap_or_default())
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

fn max_width<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    values.map(str::len).max().unwrap_or(0)
}
