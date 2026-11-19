use std::path::{Path, PathBuf};

use serde::Serialize;

use super::utils::{Place, run_checked, run_query, run_visible};
use crate::{Error, Location, Result};

const PROGRAM: &str = "git";
const ORIGIN: &str = "origin";

/// Runs `git` inside one directory, on this machine or on the one of a location.
#[derive(Debug, Clone)]
pub struct Git {
    location: Location,
    dir: PathBuf,
}

/// The tip of a branch or checkout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Commit {
    /// Abbreviated hash.
    pub sha: String,
    pub subject: String,
    /// Committer date, strict ISO 8601.
    pub date: String,
    /// Committer date relative to now, e.g. `3 days ago`.
    pub age: String,
}

/// The branches of `origin` as the last fetch left them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginBranches {
    /// The branch the remote checks out by default, when it says so.
    pub default: Option<String>,
    /// Every branch, the one with the most recent commit first.
    pub newest_first: Vec<String>,
}

/// Full name of the remote-tracking ref for `branch`.
pub fn origin_ref(branch: &str) -> String {
    format!("refs/remotes/{ORIGIN}/{branch}")
}

impl Git {
    /// `git` in a directory of this machine.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self::at(&Location::Local, dir)
    }

    pub fn at(location: &Location, dir: impl Into<PathBuf>) -> Self {
        Self {
            location: location.clone(),
            dir: dir.into(),
        }
    }

    /// Runs `git <args>` and returns its stdout. Any non-zero exit is an error.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        run_checked(PROGRAM, self.place(), args)
    }

    /// Runs a yes/no query such as `show-ref --verify --quiet`:
    /// exit 0 is `true`, exit 1 is `false`, anything else is an error.
    pub fn check(&self, args: &[&str]) -> Result<bool> {
        run_query(PROGRAM, self.place(), args)
    }

    /// Turns the directory into an empty bare repository.
    pub fn init_bare(&self) -> Result<()> {
        self.run(&["init", "--bare", "--quiet"]).map(drop)
    }

    /// Adds `origin`, fetching its branches as remote-tracking refs.
    /// `git clone --bare` would instead copy them as local branches and never update them.
    pub fn add_origin(&self, url: &str) -> Result<()> {
        self.run(&["remote", "add", "--", ORIGIN, url]).map(drop)
    }

    pub fn set_origin_url(&self, url: &str) -> Result<()> {
        self.run(&["remote", "set-url", "--", ORIGIN, url])
            .map(drop)
    }

    /// Makes a plain `git push` publish a new branch to `origin` and track it,
    /// in every worktree of this repository.
    pub fn push_new_branches_to_origin(&self) -> Result<()> {
        self.run(&["config", "push.autoSetupRemote", "true"])
            .map(drop)
    }

    /// Updates the remote-tracking refs of `origin`, dropping the ones deleted on the remote.
    pub fn fetch_origin(&self) -> Result<()> {
        self.run(&["fetch", "--prune", "--quiet", ORIGIN]).map(drop)
    }

    /// Like `fetch_origin`, but git writes its progress and its errors straight to stderr.
    /// Meant for the first, slow fetch of a clone. Not `--quiet`: that would also hide the
    /// download percentage, leaving only the server's messages.
    pub fn fetch_origin_showing_progress(&self) -> Result<()> {
        run_visible(
            PROGRAM,
            self.place(),
            &["fetch", "--prune", "--progress", ORIGIN],
        )
    }

    /// The default branch is asked to the remote: a bare clone keeps no trace of it.
    pub fn origin_branches(&self) -> Result<OriginBranches> {
        let prefix = origin_ref("");
        let refs = self.run(&[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname)",
            &prefix,
        ])?;
        let newest_first = refs
            .lines()
            .filter_map(|name| name.strip_prefix(&prefix))
            .filter(|branch| *branch != "HEAD")
            .map(str::to_string)
            .collect();
        let head = self.run(&["ls-remote", "--symref", ORIGIN, "HEAD"])?;
        let default = head
            .lines()
            .find_map(|line| {
                line.strip_prefix("ref: refs/heads/")?
                    .strip_suffix("\tHEAD")
            })
            .map(str::to_string);
        Ok(OriginBranches {
            default,
            newest_first,
        })
    }

    pub fn branch_exists(&self, branch: &str) -> Result<bool> {
        self.ref_exists(&format!("refs/heads/{branch}"))
    }

    /// Whether `branch` was on `origin` at the last fetch.
    pub fn origin_branch_exists(&self, branch: &str) -> Result<bool> {
        self.ref_exists(&origin_ref(branch))
    }

    /// Creates `branch` at `start_point` and checks it out in a new worktree at `path`.
    pub fn add_worktree(&self, path: &Path, branch: &str, start_point: &str) -> Result<()> {
        let path = path.to_string_lossy();
        self.run(&[
            "worktree",
            "add",
            "--quiet",
            "--no-track",
            "-b",
            branch,
            "--",
            &path,
            start_point,
        ])
        .map(drop)
    }

    /// Creates `branch` from the branch of `origin` with the same name, which it then
    /// follows, and checks it out in a new worktree at `path`.
    pub fn add_tracking_worktree(&self, path: &Path, branch: &str) -> Result<()> {
        let path = path.to_string_lossy();
        self.run(&[
            "worktree",
            "add",
            "--quiet",
            "--track",
            "-b",
            branch,
            "--",
            &path,
            &origin_ref(branch),
        ])
        .map(drop)
    }

    /// Makes the local `branch` follow the branch of `origin` with the same name, so a
    /// plain `git pull` and `git status` know about it.
    pub fn follow_origin_branch(&self, branch: &str) -> Result<()> {
        let upstream = format!("--set-upstream-to={}", origin_ref(branch));
        self.run(&["branch", "--quiet", &upstream, "--", branch])
            .map(drop)
    }

    /// Checks out the existing `branch` in a new worktree at `path`.
    pub fn checkout_worktree(&self, path: &Path, branch: &str) -> Result<()> {
        let path = path.to_string_lossy();
        self.run(&["worktree", "add", "--quiet", "--", &path, branch])
            .map(drop)
    }

    /// The worktree that has `branch` checked out, if any. A worktree whose directory is
    /// gone does not count: `prune_worktrees` frees its branch.
    pub fn worktree_of_branch(&self, branch: &str) -> Result<Option<PathBuf>> {
        let listed = self.run(&["worktree", "list", "--porcelain"])?;
        let checked_out = format!("branch refs/heads/{branch}");
        let worktree = listed.split("\n\n").find_map(|worktree| {
            let mut lines = worktree.lines();
            let path = lines.next()?.strip_prefix("worktree ")?;
            let rest: Vec<&str> = lines.collect();
            let holds_it = rest.contains(&checked_out.as_str());
            let gone = rest.iter().any(|line| line.starts_with("prunable"));
            (holds_it && !gone).then(|| PathBuf::from(path))
        });
        Ok(worktree)
    }

    /// Forgets the worktrees whose directory is gone, freeing their branches.
    pub fn prune_worktrees(&self) -> Result<()> {
        self.run(&["worktree", "prune"]).map(drop)
    }

    /// Deletes the worktree at `path`, uncommitted changes included.
    pub fn remove_worktree(&self, path: &Path) -> Result<()> {
        let path = path.to_string_lossy();
        self.run(&["worktree", "remove", "--force", "--", &path])
            .map(drop)
    }

    /// Deletes a local branch, unmerged commits included.
    pub fn delete_branch(&self, branch: &str) -> Result<()> {
        self.run(&["branch", "--delete", "--force", "--quiet", "--", branch])
            .map(drop)
    }

    /// The checked out branch, or `None` on a detached HEAD.
    pub fn current_branch(&self) -> Result<Option<String>> {
        let branch = self.run(&["branch", "--show-current"])?;
        let branch = branch.trim_end();
        Ok((!branch.is_empty()).then(|| branch.to_string()))
    }

    /// The commit `rev` points to. Pass a full ref name or `HEAD`.
    pub fn last_commit(&self, rev: &str) -> Result<Commit> {
        let raw = self.run(&["log", "-1", "--format=%h%x00%s%x00%cI%x00%cr", rev, "--"])?;
        let mut fields = raw.trim_end().split('\0').map(str::to_string);
        let mut next = || fields.next().unwrap_or_default();
        Ok(Commit {
            sha: next(),
            subject: next(),
            date: next(),
            age: next(),
        })
    }

    /// Whether the working tree has modified, staged or untracked files.
    pub fn has_changes(&self) -> Result<bool> {
        let changes = self.run(&["status", "--porcelain"])?;
        Ok(!changes.is_empty())
    }

    /// How many commits of `rev` are on no branch of `origin`, as of the last fetch or push.
    pub fn unpushed_commits(&self, rev: &str) -> Result<usize> {
        let args = ["rev-list", "--count", rev, "--not", "--remotes=origin"];
        let count = self.run(&args)?;
        count.trim().parse().map_err(|_| Error::UnexpectedOutput {
            command: format!("{PROGRAM} {}", args.join(" ")),
            output: count.trim().to_string(),
        })
    }

    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool> {
        self.check(&["merge-base", "--is-ancestor", ancestor, descendant])
    }

    /// Moves the checked out branch forward to `rev`. Fails rather than merge or rebase.
    pub fn fast_forward(&self, rev: &str) -> Result<()> {
        self.run(&["merge", "--ff-only", "--quiet", rev]).map(drop)
    }

    /// Compares the names itself: on a disk that ignores case, git finds `Feat/Login`
    /// when asked for `feat/login`.
    fn ref_exists(&self, full_name: &str) -> Result<bool> {
        let refs = self.run(&["for-each-ref", "--format=%(refname)", full_name])?;
        Ok(refs.lines().any(|name| name == full_name))
    }

    fn place(&self) -> Place<'_> {
        Place::machine(&self.location).in_dir(&self.dir)
    }
}
