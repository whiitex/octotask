use octotask_core::{Error, Gh};
use tempfile::TempDir;

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
