//! Tests for `src/workspaces/`, one file per source file, plus the fixtures they share:
//! local git repos used as remotes, with no network.

mod repos;
mod workspace;

use std::path::{Path, PathBuf};
use std::process::Command;

use octotask_core::{Home, NewRepo, Project, Registry, register_repo};
use tempfile::TempDir;

const PROJECT: &str = "deeptree";

struct Fixture {
    tmp: TempDir,
}

impl Fixture {
    /// An empty home with one project whose base is `main`.
    fn new() -> Self {
        let fixture = Self {
            tmp: TempDir::new().unwrap(),
        };
        fixture.registry().init_project(PROJECT, "main").unwrap();
        fixture
    }

    fn home(&self) -> Home {
        Home::new(self.home_dir())
    }

    fn home_dir(&self) -> PathBuf {
        self.tmp.path().join("home")
    }

    fn registry(&self) -> Registry {
        Registry::new(self.home())
    }

    fn project(&self) -> Project {
        self.registry().load_project(PROJECT).unwrap()
    }

    fn remote_dir(&self, name: &str) -> PathBuf {
        self.tmp.path().join("remotes").join(format!("{name}.git"))
    }

    /// Creates a repo with one commit on `main` and returns its url.
    fn remote(&self, name: &str) -> String {
        let dir = self.remote_dir(name);
        std::fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "--quiet", "--initial-branch", "main"]);
        commit_here(&dir, "init");
        dir.display().to_string()
    }

    /// Registers the repo at `url` with the default key and base, and returns its key.
    fn register(&self, url: &str) -> String {
        let repo = NewRepo {
            url,
            ..Default::default()
        };
        register_repo(&self.registry(), PROJECT, &repo, refuse_missing_base).unwrap()
    }

    fn cache_dir(&self, repo: &str) -> PathBuf {
        self.home_dir()
            .join("cache")
            .join(PROJECT)
            .join(format!("{repo}.git"))
    }

    fn workspace_dir(&self, name: &str) -> PathBuf {
        self.home_dir().join("workspaces").join(PROJECT).join(name)
    }
}

/// Runs plain `git`, independently of the code under test, and returns its trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Commits on the branch that is checked out in `dir`.
fn commit_here(dir: &Path, message: &str) {
    git(
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
            message,
        ],
    );
}

/// Commits on `branch`, creating it from the current one when it does not exist.
fn commit_on(dir: &Path, branch: &str, message: &str) {
    let exists = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/heads/{branch}"))
        .status()
        .unwrap()
        .success();
    if exists {
        git(dir, &["checkout", "--quiet", branch]);
    } else {
        git(dir, &["checkout", "--quiet", "-b", branch]);
    }
    commit_here(dir, message);
}

fn head(dir: &Path, rev: &str) -> String {
    git(dir, &["rev-parse", rev])
}

/// Answer to "use a base branch that is not on the remote?" for tests that never expect the question.
fn refuse_missing_base(_repo: &str, _base: &str) -> bool {
    false
}
