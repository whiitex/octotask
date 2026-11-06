//! Tests for `src/adapters/`, one file per source file, plus the fixtures they share.

mod gh;
mod git;
mod utils;

use std::path::Path;
use std::process::Command;

fn init_repo(dir: &Path) {
    git_setup(dir, &["init", "--quiet"]);
}

fn commit_empty(dir: &Path) {
    git_setup(
        dir,
        &[
            "-c",
            "user.name=octok",
            "-c",
            "user.email=octok@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "--no-verify",
            "--quiet",
            "--message",
            "init",
        ],
    );
}

fn canonical(path: &Path) -> String {
    path.canonicalize().unwrap().display().to_string()
}

/// Prepares fixtures with plain `git`, independently of the adapters under test.
fn git_setup(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}
