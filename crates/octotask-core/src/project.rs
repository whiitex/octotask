use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::utils::CONFIG_VERSION;
use crate::{Error, Result};

/// A project: the set of repos that make up one product, plus their default base branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub base: String,
    #[serde(default)]
    pub repos: IndexMap<String, Repo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
}

impl Project {
    pub fn new(name: &str, base: &str) -> Result<Self> {
        validate_ident("project name", name)?;
        validate_branch(base)?;
        Ok(Self {
            version: CONFIG_VERSION,
            name: name.to_string(),
            base: base.to_string(),
            repos: IndexMap::new(),
        })
    }

    /// Registers a repo. The key defaults to the last path segment of the url.
    /// Returns the key the repo was registered under.
    pub fn add_repo(&mut self, url: &str, key: Option<&str>, base: Option<&str>) -> Result<String> {
        let derived = repo_key_from_url(url)?;
        let key = match key {
            Some(k) => {
                validate_ident("repo key", k)?;
                k.to_string()
            }
            None => derived,
        };
        if let Some(b) = base {
            validate_branch(b)?;
        }
        if self.repos.contains_key(&key) {
            return Err(Error::DuplicateRepo(key));
        }
        self.repos.insert(
            key.clone(),
            Repo {
                url: url.trim().to_string(),
                base: base.map(str::to_string),
            },
        );
        Ok(key)
    }

    pub fn remove_repo(&mut self, key: &str) -> Result<Repo> {
        self.repos
            .shift_remove(key)
            .ok_or_else(|| Error::RepoNotFound(key.to_string()))
    }

    /// Base branch for a repo: its own override, else the project base.
    pub fn effective_base<'a>(&'a self, repo: &'a Repo) -> &'a str {
        repo.base.as_deref().unwrap_or(&self.base)
    }

    /// Checks invariants that serde cannot express, e.g. after loading a hand-edited file.
    pub(crate) fn validate(&self) -> Result<()> {
        validate_ident("project name", &self.name)?;
        validate_branch(&self.base)?;
        for (key, repo) in &self.repos {
            validate_ident("repo key", key)?;
            repo_key_from_url(&repo.url)?;
            if let Some(b) = &repo.base {
                validate_branch(b)?;
            }
        }
        Ok(())
    }
}

/// Derives a repo key from a git url: `git@github.com:org/frontend.git` gives `frontend`.
pub fn repo_key_from_url(url: &str) -> Result<String> {
    let invalid = || Error::InvalidUrl(url.to_string());
    let path = url_path(url.trim()).ok_or_else(invalid)?;
    let key = repo_name_from_path(path);
    validate_ident("repo key", key).map_err(|_| invalid())?;
    Ok(key.to_string())
}

fn url_path(url: &str) -> Option<&str> {
    if url.is_empty() || url.chars().any(char::is_whitespace) {
        return None;
    }
    if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(scheme, "https" | "http" | "ssh" | "git" | "file") {
            return None;
        }
        // `rest` is `host[:port]/path`, or `/path` for file urls.
        return rest.split_once('/').map(|(_host, path)| path);
    }
    if url.starts_with('/') {
        return Some(url);
    }
    // scp-like syntax: `[user@]host:path`.
    let (host, path) = url.split_once(':')?;
    if host.is_empty() || host.contains('/') {
        return None;
    }
    Some(path)
}

fn repo_name_from_path(path: &str) -> &str {
    let last = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    last.strip_suffix(".git").unwrap_or(last)
}

/// Names used as file and directory names: project names and repo keys.
pub(crate) fn validate_ident(kind: &'static str, value: &str) -> Result<()> {
    let fail = |reason| {
        Err(Error::InvalidName {
            kind,
            value: value.to_string(),
            reason,
        })
    };
    let Some(first) = value.chars().next() else {
        return fail("must not be empty");
    };
    if value.len() > 64 {
        return fail("must be at most 64 characters");
    }
    if !first.is_ascii_alphanumeric() {
        return fail("must start with a letter or digit");
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return fail("may only contain letters, digits, `.`, `_` and `-`");
    }
    Ok(())
}

/// A cheap subset of `git check-ref-format --branch`, enough to catch typos early.
fn validate_branch(value: &str) -> Result<()> {
    let fail = |reason| {
        Err(Error::InvalidName {
            kind: "branch",
            value: value.to_string(),
            reason,
        })
    };
    if value.is_empty() {
        return fail("must not be empty");
    }
    if value.starts_with('-') {
        return fail("must not start with `-`");
    }
    if value
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || "~^:?*[\\".contains(c))
    {
        return fail("contains a character git does not allow");
    }
    if value.contains("..") || value.ends_with('/') || value.ends_with(".lock") {
        return fail("is not a valid git branch name");
    }
    Ok(())
}
