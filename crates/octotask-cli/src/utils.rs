use std::io::{BufRead, Write};
use std::process::ExitCode;

use octotask_core::{Commit, Project, PullOutcome, Result, Status};
use serde_json::json;

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
}

fn print_repos_json(project_name: &str, rows: &[RepoRow]) {
    let repos: Vec<_> = rows
        .iter()
        .map(|row| json!({"key": row.key, "alias": row.alias, "url": row.url, "base": row.base}))
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
    for (row, label) in rows.iter().zip(labels) {
        println!(
            "{label:label_width$}  {:base_width$}  {}",
            row.base, row.url
        );
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
        println!(
            "  {label:label_width$}  {:base_width$}  {commit}",
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
        let labels: Vec<String> = workspace
            .checkouts
            .iter()
            .map(|checkout| status_repo_label(status, &checkout.repo))
            .collect();
        let branches: Vec<&str> = workspace
            .checkouts
            .iter()
            .map(|checkout| checkout.branch.as_deref().unwrap_or("(detached)"))
            .collect();
        let label_width = max_width(labels.iter().map(String::as_str));
        let branch_width = max_width(branches.iter().copied());
        let sha_width = sha_width(workspace.checkouts.iter().map(|c| &c.commit));
        for ((checkout, label), branch) in workspace.checkouts.iter().zip(labels).zip(branches) {
            println!(
                "    {label:label_width$}  {branch:branch_width$}  {}",
                commit_line(&checkout.commit, sha_width)
            );
        }
    }
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
    let alias = project
        .repos
        .get(key)
        .and_then(|repo| repo.alias.as_deref());
    repo_label(key, alias)
}

fn status_repo_label(status: &Status, key: &str) -> String {
    let alias = status
        .repos
        .iter()
        .find(|repo| repo.key == key)
        .and_then(|repo| repo.alias.as_deref());
    repo_label(key, alias)
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
