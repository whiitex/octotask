use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use octotask_core::{Error, NewRepo, UpEvent, create_workspace, register_repo, start_workspace};

use super::{Fixture, PROJECT, keep_project_base, refuse_missing_base};

const WORKSPACE: &str = "feat-login";

/// Prints `ready` once it has started a `sleep` and written its pid to `sleep.pid`.
const SLEEP_IN_BACKGROUND: &str = "sleep 300 & echo $! > sleep.pid; echo ready; wait";

/// The workspace `feat-login` over these repos, each with its run command if it has one.
fn workspace_with_commands(repos: &[(&str, Option<&str>)]) -> Fixture {
    let fx = Fixture::new();
    for (name, _) in repos {
        fx.register(&fx.remote(name));
    }
    fx.registry()
        .update_project(PROJECT, |project| {
            repos
                .iter()
                .try_for_each(|(name, run)| project.set_repo_run(name, *run))
        })
        .unwrap();
    create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();
    fx
}

fn set_setup(fx: &Fixture, repo: &str, setup: &str) {
    fx.registry()
        .update_project(PROJECT, |project| project.set_repo_setup(repo, Some(setup)))
        .unwrap();
}

fn checkout(fx: &Fixture, repo: &str) -> PathBuf {
    fx.workspace_dir(WORKSPACE).join(repo)
}

#[derive(Default)]
struct Seen {
    lines: Vec<(String, String)>,
    exits: Vec<(String, ExitStatus)>,
}

impl Seen {
    fn record(&mut self, event: UpEvent) {
        match event {
            UpEvent::Output { repo, line } => self.lines.push((repo.to_string(), line.to_string())),
            UpEvent::Exited { repo, status } => self.exits.push((repo.to_string(), status)),
        }
    }

    /// Every line shown for the repo: the commands octok runs, each after `$ `, and what they print.
    fn lines_of(&self, repo: &str) -> Vec<&str> {
        self.lines
            .iter()
            .filter(|(owner, _)| owner == repo)
            .map(|(_, line)| line.as_str())
            .collect()
    }

    /// What the commands of the repo printed.
    fn printed_by(&self, repo: &str) -> Vec<&str> {
        let mut lines = self.lines_of(repo);
        lines.retain(|line| !line.starts_with("$ "));
        lines
    }

    fn exit_of(&self, repo: &str) -> ExitStatus {
        let exits: Vec<_> = self.exits.iter().filter(|(key, _)| key == repo).collect();
        assert_eq!(exits.len(), 1, "exits of {repo}: {:?}", self.exits);
        exits[0].1
    }
}

/// Starts the workspace and waits for every command to end, stopping them all at the first
/// `ready` line when `stop_when_ready`.
fn run_to_the_end(fx: &Fixture, stop_when_ready: bool) -> (bool, Seen) {
    let running = start_workspace(&fx.home(), &fx.project(), WORKSPACE, &[]).unwrap();
    let stopper = running.stopper();
    let mut seen = Seen::default();
    let all_succeeded = running.wait(|event| {
        if stop_when_ready && matches!(event, UpEvent::Output { line: "ready", .. }) {
            stopper.stop();
        }
        seen.record(event);
    });
    (all_succeeded, seen)
}

fn start_error(fx: &Fixture, repos: &[&str]) -> Error {
    start_workspace(&fx.home(), &fx.project(), WORKSPACE, repos)
        .err()
        .unwrap()
}

fn is_running(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

/// The process whose pid is in `pid_file` is gone, at the latest a moment from now.
fn assert_stopped(pid_file: &Path) {
    let pid = fs::read_to_string(pid_file).unwrap().trim().to_string();
    let deadline = Instant::now() + Duration::from_secs(5);
    while is_running(&pid) {
        assert!(Instant::now() < deadline, "process {pid} is still running");
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn up_runs_each_command_in_its_checkout_and_reports_output_and_exit() {
    let fx = workspace_with_commands(&[
        (
            "frontend",
            Some(r#"echo "out in $(basename "$PWD")"; echo err >&2"#),
        ),
        ("backend", Some("pwd -P")),
        ("docs", None),
    ]);

    let running = start_workspace(&fx.home(), &fx.project(), WORKSPACE, &[]).unwrap();
    let started: Vec<&str> = running.started().collect();
    assert_eq!(started, ["frontend", "backend"]);
    let mut seen = Seen::default();
    let all_succeeded = running.wait(|event| seen.record(event));

    assert!(all_succeeded);
    assert_eq!(
        seen.lines_of("frontend"),
        [
            r#"$ echo "out in $(basename "$PWD")"; echo err >&2"#,
            "out in frontend",
            "err"
        ]
    );
    let backend = checkout(&fx, "backend").canonicalize().unwrap();
    assert_eq!(seen.printed_by("backend"), [backend.to_str().unwrap()]);
    assert!(seen.exit_of("frontend").success());
    assert!(seen.exit_of("backend").success());
    assert!(seen.lines_of("docs").is_empty());
    assert_eq!(seen.exits.len(), 2);
}

#[test]
fn up_runs_the_command_of_the_workspace_instead_of_the_one_of_the_repo() {
    let fx = workspace_with_commands(&[("frontend", Some("echo from the repo"))]);
    fx.registry()
        .update_project(PROJECT, |project| {
            project.set_workspace_run(WORKSPACE, "frontend", Some("echo from the workspace"))
        })
        .unwrap();

    let (_, seen) = run_to_the_end(&fx, false);

    assert_eq!(seen.printed_by("frontend"), ["from the workspace"]);
}

#[test]
fn up_starts_only_the_repos_it_is_given_by_key_or_alias() {
    let fx = workspace_with_commands(&[
        ("frontend", Some("echo web")),
        ("backend", Some("echo api")),
        ("agent", Some("echo agent")),
    ]);
    fx.registry()
        .update_project(PROJECT, |project| project.set_alias("backend", "api"))
        .unwrap();

    let running =
        start_workspace(&fx.home(), &fx.project(), WORKSPACE, &["api", "frontend"]).unwrap();
    let started: Vec<String> = running.started().map(str::to_string).collect();
    let mut seen = Seen::default();
    running.wait(|event| seen.record(event));

    assert_eq!(started, ["frontend", "backend"]);
    assert_eq!(seen.exits.len(), 2);
    assert!(seen.lines_of("agent").is_empty());
}

#[test]
fn a_repo_given_to_up_must_exist_and_have_a_run_command() {
    let fx = workspace_with_commands(&[("frontend", Some("echo web")), ("docs", None)]);

    let err = start_error(&fx, &["frontend", "docs"]);
    assert!(
        matches!(&err, Error::NoRunCommand(repo) if repo == "docs"),
        "{err:?}"
    );
    assert!(
        err.to_string().contains("octok repo set docs --run"),
        "{err}"
    );

    let err = start_error(&fx, &["nope"]);
    assert!(matches!(err, Error::RepoNotFound(_)), "{err:?}");
}

#[test]
fn a_failing_command_is_reported_and_does_not_stop_the_others() {
    let fx = workspace_with_commands(&[
        ("frontend", Some("exit 3")),
        ("backend", Some("sleep 1; echo still here")),
    ]);

    let (all_succeeded, seen) = run_to_the_end(&fx, false);

    assert!(!all_succeeded);
    assert_eq!(seen.exit_of("frontend").code(), Some(3));
    assert_eq!(seen.printed_by("backend"), ["still here"]);
    assert!(seen.exit_of("backend").success());
}

#[test]
fn setup_runs_before_the_first_run_and_again_only_when_it_changes() {
    let fx = workspace_with_commands(&[("frontend", Some("echo run"))]);
    set_setup(&fx, "frontend", "echo installed >> setup.log");
    let setups = || fs::read_to_string(checkout(&fx, "frontend").join("setup.log")).unwrap();

    let (all_succeeded, seen) = run_to_the_end(&fx, false);
    assert!(all_succeeded);
    assert_eq!(
        seen.lines_of("frontend"),
        ["$ echo installed >> setup.log", "$ echo run", "run"]
    );

    let (_, seen) = run_to_the_end(&fx, false);
    assert_eq!(seen.lines_of("frontend"), ["$ echo run", "run"]);
    assert_eq!(setups(), "installed\n");

    set_setup(&fx, "frontend", "echo installed again >> setup.log");
    run_to_the_end(&fx, false);
    assert_eq!(setups(), "installed\ninstalled again\n");
}

#[test]
fn a_failing_setup_keeps_the_repo_from_running_and_is_tried_again() {
    let fx = workspace_with_commands(&[("frontend", Some("echo run"))]);
    set_setup(&fx, "frontend", "echo try >> tries.log; exit 4");

    let (all_succeeded, seen) = run_to_the_end(&fx, false);
    assert!(!all_succeeded);
    assert_eq!(seen.exit_of("frontend").code(), Some(4));
    assert!(seen.printed_by("frontend").is_empty());

    run_to_the_end(&fx, false);
    let tries = fs::read_to_string(checkout(&fx, "frontend").join("tries.log")).unwrap();
    assert_eq!(tries, "try\ntry\n");
}

#[test]
fn stop_ends_every_command_and_the_processes_it_started() {
    let fx = workspace_with_commands(&[
        ("frontend", Some(SLEEP_IN_BACKGROUND)),
        ("backend", Some("sleep 300")),
    ]);
    let started = Instant::now();

    let (all_succeeded, seen) = run_to_the_end(&fx, true);

    assert!(!all_succeeded);
    assert!(!seen.exit_of("frontend").success());
    assert!(!seen.exit_of("backend").success());
    assert!(started.elapsed() < Duration::from_secs(8), "stop was slow");
    assert_stopped(&checkout(&fx, "frontend").join("sleep.pid"));
}

#[test]
fn stop_during_the_setup_does_not_start_the_run_command() {
    let fx = workspace_with_commands(&[("frontend", Some("echo run"))]);
    set_setup(&fx, "frontend", "echo ready; sleep 300");

    let (all_succeeded, seen) = run_to_the_end(&fx, true);

    assert!(!all_succeeded);
    assert_eq!(seen.printed_by("frontend"), ["ready"]);
}

#[test]
fn a_command_that_ignores_the_stop_is_killed_after_ten_seconds() {
    let ignore_the_stop =
        "trap '' TERM; echo $$ > shell.pid; echo ready; while :; do sleep 1; done";
    let fx = workspace_with_commands(&[("frontend", Some(ignore_the_stop))]);
    let started = Instant::now();

    let (all_succeeded, seen) = run_to_the_end(&fx, true);

    assert!(!all_succeeded);
    assert_eq!(seen.exit_of("frontend").signal(), Some(9));
    assert!(
        started.elapsed() >= Duration::from_secs(9),
        "killed too soon"
    );
    assert_stopped(&checkout(&fx, "frontend").join("shell.pid"));
}

#[test]
fn dropping_a_running_workspace_stops_its_commands() {
    let fx = workspace_with_commands(&[("frontend", Some(SLEEP_IN_BACKGROUND))]);
    let pid_file = checkout(&fx, "frontend").join("sleep.pid");

    let running = start_workspace(&fx.home(), &fx.project(), WORKSPACE, &[]).unwrap();
    while !pid_file.exists() {
        thread::sleep(Duration::from_millis(20));
    }
    drop(running);

    assert_stopped(&pid_file);
}

#[test]
fn up_is_refused_for_a_workspace_that_does_not_exist() {
    let fx = workspace_with_commands(&[("frontend", Some("echo hi"))]);

    let err = start_workspace(&fx.home(), &fx.project(), "feat-other", &[])
        .err()
        .unwrap();

    assert!(
        matches!(&err, Error::WorkspaceNotFound(name) if name == "feat-other"),
        "{err:?}"
    );
}

#[test]
fn up_is_refused_when_no_repo_has_a_run_command() {
    let fx = workspace_with_commands(&[("frontend", None)]);

    let err = start_error(&fx, &[]);

    assert!(matches!(err, Error::NothingToRun(_)), "{err:?}");
    assert!(err.to_string().contains("octok repo set"), "{err}");
}

#[test]
fn up_starts_nothing_when_a_repo_to_run_has_no_worktree() {
    let fx = workspace_with_commands(&[("frontend", Some("echo started > started"))]);
    let late = NewRepo {
        url: &fx.remote("backend"),
        ..Default::default()
    };
    register_repo(
        &fx.registry(),
        PROJECT,
        &late,
        refuse_missing_base,
        keep_project_base,
    )
    .unwrap();
    fx.registry()
        .update_project(PROJECT, |project| {
            project.set_repo_run("backend", Some("echo hi"))
        })
        .unwrap();

    let err = start_error(&fx, &[]);

    assert!(
        matches!(&err, Error::NoWorktree { repo, .. } if repo == "backend"),
        "{err:?}"
    );
    assert!(!checkout(&fx, "frontend").join("started").exists());
}
