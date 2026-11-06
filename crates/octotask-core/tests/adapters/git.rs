use octotask_core::{Error, Git};
use tempfile::TempDir;

use super::{canonical, commit_empty, init_repo};

#[test]
fn run_returns_stdout_from_the_given_directory() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    let toplevel = Git::new(dir.path())
        .run(&["rev-parse", "--show-toplevel"])
        .unwrap();

    assert_eq!(toplevel.trim_end(), canonical(dir.path()));
}

#[test]
fn check_maps_exit_0_to_true_and_exit_1_to_false() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());
    let git = Git::new(dir.path());
    let head_exists = ["rev-parse", "--verify", "--quiet", "HEAD"];

    assert!(!git.check(&head_exists).unwrap());
    commit_empty(dir.path());
    assert!(git.check(&head_exists).unwrap());
}

#[test]
fn check_reports_other_exit_codes_as_errors() {
    let dir = TempDir::new().unwrap();

    let err = Git::new(dir.path())
        .check(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .unwrap_err();

    assert!(
        matches!(&err, Error::CommandFailed { status, .. } if status.code() == Some(128)),
        "{err:?}"
    );
}
