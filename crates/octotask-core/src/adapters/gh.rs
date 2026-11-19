use std::path::PathBuf;

use serde::Deserialize;

use super::utils::{Place, run_checked};
use crate::{Error, Result};

const PROGRAM: &str = "gh";

/// What `gh --json` is asked for about a pull request.
const PULL_REQUEST_FIELDS: &str = "number,title,headRefName,author,isCrossRepository";

/// How many open pull requests of a repo are listed, the newest first.
const LISTED_PULL_REQUESTS: &str = "100";

/// Runs the system GitHub CLI `gh` inside one directory.
#[derive(Debug, Clone)]
pub struct Gh {
    dir: PathBuf,
}

/// A pull request, as far as choosing what to review needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    /// The branch the pull request wants merged.
    pub head_branch: String,
    /// Login of who opened it. Empty for an account that no longer exists.
    pub author: String,
    /// Whether `head_branch` is in another repository, not in the one of the pull request.
    pub from_fork: bool,
}

impl Gh {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Runs `gh <args>` and returns its stdout. Any non-zero exit is an error.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        run_checked(PROGRAM, Place::local(&self.dir), args)
    }

    /// The open pull requests of the repo at `repo_url`, the newest first.
    pub fn open_pull_requests(&self, repo_url: &str) -> Result<Vec<PullRequest>> {
        let args = [
            "pr",
            "list",
            "--repo",
            repo_url,
            "--state",
            "open",
            "--limit",
            LISTED_PULL_REQUESTS,
            "--json",
            PULL_REQUEST_FIELDS,
        ];
        let listed: Vec<RawPullRequest> = self.run_for_json(&args)?;
        Ok(listed.into_iter().map(PullRequest::from).collect())
    }

    /// The pull request `number` of the repo at `repo_url`, whatever its state.
    pub fn pull_request(&self, repo_url: &str, number: u64) -> Result<PullRequest> {
        let number = number.to_string();
        let args = [
            "pr",
            "view",
            &number,
            "--repo",
            repo_url,
            "--json",
            PULL_REQUEST_FIELDS,
        ];
        let found: RawPullRequest = self.run_for_json(&args)?;
        Ok(found.into())
    }

    fn run_for_json<T: for<'de> Deserialize<'de>>(&self, args: &[&str]) -> Result<T> {
        let output = self.run(args)?;
        serde_json::from_str(&output).map_err(|_| Error::UnexpectedOutput {
            command: format!("{PROGRAM} {}", args.join(" ")),
            output: output.trim().to_string(),
        })
    }
}

/// A pull request with the field names of `gh --json`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPullRequest {
    number: u64,
    title: String,
    head_ref_name: String,
    author: Option<RawAuthor>,
    is_cross_repository: bool,
}

#[derive(Deserialize)]
struct RawAuthor {
    login: String,
}

impl From<RawPullRequest> for PullRequest {
    fn from(raw: RawPullRequest) -> Self {
        Self {
            number: raw.number,
            title: raw.title,
            head_branch: raw.head_ref_name,
            author: raw.author.map(|author| author.login).unwrap_or_default(),
            from_fork: raw.is_cross_repository,
        }
    }
}
