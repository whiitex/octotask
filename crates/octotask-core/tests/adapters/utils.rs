//! Tests for the runner shared by the adapters, exercised through `Git` and `Gh`.

use std::env;
use std::ffi::OsStr;
use std::process::Command;

use octotask_core::{Error, Gh, Git};
use tempfile::TempDir;

use super::init_repo;

const CHILD_MARKER: &str = "OCTOTASK_TEST_CHILD";
const TARGET_REPO: &str = "OCTOTASK_TEST_TARGET_REPO";

#[test]
fn failure_reports_command_directory_status_and_stderr() {
    let dir = TempDir::new().unwrap();

    let err = Git::new(dir.path()).run(&["status"]).unwrap_err();

    let Error::CommandFailed { status, stderr, .. } = &err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(status.code(), Some(128));
    assert!(!stderr.is_empty());
    assert_eq!(
        err.to_string(),
        format!(
            "`git status` failed in {} (exit status: 128)\n{stderr}",
            dir.path().display()
        )
    );
}

#[test]
fn missing_directory_is_not_reported_as_missing_tool() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing");

    let err = Git::new(&missing).run(&["status"]).unwrap_err();

    assert!(
        matches!(&err, Error::Spawn { program: "git", dir, .. } if *dir == missing),
        "{err:?}"
    );
}

#[test]
fn credentials_in_urls_are_hidden_from_errors() {
    let dir = TempDir::new().unwrap();

    // Port 1 refuses connections, so this fails fast without network access.
    let err = Git::new(dir.path())
        .run(&["ls-remote", "https://user:secret@127.0.0.1:1/org/repo.git"])
        .unwrap_err();

    let message = err.to_string();
    assert!(!message.contains("secret"), "{message}");
    assert!(
        message.starts_with("`git ls-remote https://***@127.0.0.1:1/org/repo.git` failed"),
        "{message}"
    );
}

#[test]
fn git_dir_of_the_caller_does_not_redirect_git() {
    if is_child() {
        let target = env::var(TARGET_REPO).unwrap();
        let toplevel = Git::new(&target)
            .run(&["rev-parse", "--show-toplevel"])
            .unwrap();
        assert_eq!(toplevel.trim_end(), target);
        return;
    }
    let target = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    init_repo(target.path());
    init_repo(other.path());
    let target_path = target.path().canonicalize().unwrap();
    let other_git_dir = other.path().join(".git");

    run_in_child(
        "git_dir_of_the_caller_does_not_redirect_git",
        &[
            ("GIT_DIR", other_git_dir.as_os_str()),
            ("GIT_WORK_TREE", other.path().as_os_str()),
            (TARGET_REPO, target_path.as_os_str()),
        ],
    );
}

#[test]
fn git_runs_with_terminal_prompts_disabled() {
    if is_child() {
        let dir = TempDir::new().unwrap();
        let value = Git::new(dir.path())
            .run(&[
                "-c",
                "alias.prompt-setting=!printenv GIT_TERMINAL_PROMPT",
                "prompt-setting",
            ])
            .unwrap();
        assert_eq!(value, "0\n");
        return;
    }
    run_in_child(
        "git_runs_with_terminal_prompts_disabled",
        &[("GIT_TERMINAL_PROMPT", OsStr::new("1"))],
    );
}

#[test]
fn missing_binaries_are_reported_by_name() {
    if is_child() {
        let dir = TempDir::new().unwrap();
        let git_err = Git::new(dir.path()).run(&["status"]).unwrap_err();
        let gh_err = Gh::new(dir.path()).run(&["--version"]).unwrap_err();
        assert!(matches!(git_err, Error::ToolNotFound("git")), "{git_err:?}");
        assert!(matches!(gh_err, Error::ToolNotFound("gh")), "{gh_err:?}");
        assert_eq!(
            git_err.to_string(),
            "`git` not found: install it or add it to PATH"
        );
        return;
    }
    run_in_child(
        "missing_binaries_are_reported_by_name",
        &[("PATH", OsStr::new("/nonexistent"))],
    );
}

fn is_child() -> bool {
    env::var_os(CHILD_MARKER).is_some()
}

/// Runs one test of this file again, in a child copy of the test binary with `vars` added,
/// so the environment change cannot leak into tests running in parallel.
/// The child takes the test's `is_child()` branch.
fn run_in_child(test_name: &str, vars: &[(&str, &OsStr)]) {
    let module = module_path!()
        .split_once("::")
        .map_or("", |(_crate, module)| module);
    let output = Command::new(env::current_exe().unwrap())
        .args([format!("{module}::{test_name}").as_str(), "--exact"])
        .env(CHILD_MARKER, "1")
        .envs(vars.iter().copied())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "child run of {test_name} failed:\n{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
