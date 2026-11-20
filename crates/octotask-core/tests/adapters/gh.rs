use octotask_core::{Error, Gh, PullRequest};
use tempfile::TempDir;

use super::{FakeGh, is_child};

const BACKEND: &str = "git@github.com:acme/shop-backend.git";

const LOGIN: &str = r#"{"author":{"id":"MDQ6","is_bot":false,"login":"alice","name":"Alice"},
    "headRefName":"feat/login","isCrossRepository":false,"number":12,"title":"Add \"login\""}"#;

const FROM_A_FORK: &str = r#"{"author":{"login":"mallory"},"headRefName":"patch-1",
    "isCrossRepository":true,"number":13,"title":"Fix typo"}"#;

fn login() -> PullRequest {
    PullRequest {
        number: 12,
        title: "Add \"login\"".to_string(),
        head_branch: "feat/login".to_string(),
        author: "alice".to_string(),
        from_fork: false,
    }
}

fn gh() -> Gh {
    Gh::new(std::env::temp_dir())
}

#[test]
fn run_returns_stdout() {
    let dir = TempDir::new().unwrap();

    let version = Gh::new(dir.path()).run(&["--version"]).unwrap();

    assert!(version.starts_with("gh version "), "{version}");
}

#[test]
fn failure_reports_command_and_stderr() {
    let dir = TempDir::new().unwrap();

    let err = Gh::new(dir.path()).run(&["--no-such-flag"]).unwrap_err();

    let Error::CommandFailed { status, stderr, .. } = &err else {
        panic!("unexpected error: {err:?}");
    };
    assert_eq!(status.code(), Some(1));
    assert!(stderr.contains("--no-such-flag"), "{stderr}");
    assert!(
        err.to_string()
            .starts_with("`gh --no-such-flag` failed in "),
        "{err}"
    );
}

#[test]
fn open_pull_requests_of_a_repo_are_listed() {
    if is_child() {
        let listed = gh().open_pull_requests(BACKEND).unwrap();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0], login());
        assert_eq!(
            (listed[1].number, listed[1].from_fork),
            (13, true),
            "{listed:?}"
        );
        let none = gh().open_pull_requests("git@github.com:acme/shop-frontend.git");
        assert_eq!(none.unwrap(), []);
        return;
    }
    let fake = FakeGh::new();
    fake.pull_request("shop-backend", 12, LOGIN);
    fake.pull_request("shop-backend", 13, FROM_A_FORK);
    fake.run_in_child(module_path!(), "open_pull_requests_of_a_repo_are_listed");
}

#[test]
fn a_pull_request_is_read_by_its_number() {
    if is_child() {
        assert_eq!(gh().pull_request(BACKEND, 12).unwrap(), login());

        let err = gh().pull_request(BACKEND, 99).unwrap_err();
        let Error::CommandFailed { stderr, .. } = &err else {
            panic!("unexpected error: {err:?}");
        };
        assert!(stderr.contains("Could not resolve"), "{stderr}");
        return;
    }
    let fake = FakeGh::new();
    fake.pull_request("shop-backend", 12, LOGIN);
    fake.run_in_child(module_path!(), "a_pull_request_is_read_by_its_number");
}

#[test]
fn an_answer_of_gh_that_is_not_a_pull_request_is_reported() {
    if is_child() {
        let err = gh().pull_request(BACKEND, 12).unwrap_err();

        assert!(
            matches!(&err, Error::UnexpectedOutput { output, .. } if output.contains("oops")),
            "{err:?}"
        );
        return;
    }
    let fake = FakeGh::new();
    fake.pull_request("shop-backend", 12, r#"{"oops":true}"#);
    fake.run_in_child(
        module_path!(),
        "an_answer_of_gh_that_is_not_a_pull_request_is_reported",
    );
}
