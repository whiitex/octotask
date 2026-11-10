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
    /// Short name accepted wherever a repo key is, e.g. `frontend`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
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
        validated_url_path(url)?;
        let key = match key {
            Some(k) => {
                validate_ident("repo key", k)?;
                k.to_string()
            }
            None => repo_key_from_url(url)?,
        };
        if let Some(b) = base {
            validate_branch(b)?;
        }
        if let Some(existing) = self.repos.get(&key) {
            return Err(Error::DuplicateRepo {
                key,
                url: existing.url.clone(),
            });
        }
        if let Some((owner, _)) = self
            .repos
            .iter()
            .find(|(_, repo)| repo.alias.as_deref() == Some(key.as_str()))
        {
            return Err(Error::DuplicateAlias {
                alias: key,
                repo: owner.clone(),
            });
        }
        self.repos.insert(
            key.clone(),
            Repo {
                alias: None,
                url: url.trim().to_string(),
                base: base.map(str::to_string),
            },
        );
        Ok(key)
    }

    /// Gives a registered repo a short name. Keys and aliases share one namespace.
    pub fn set_alias(&mut self, key: &str, alias: &str) -> Result<()> {
        if !self.repos.contains_key(key) {
            return Err(Error::RepoNotFound(key.to_string()));
        }
        validate_ident("alias", alias)?;
        self.ensure_alias_is_free(key, alias)?;
        self.repos[key].alias = Some(alias.to_string());
        Ok(())
    }

    /// Key of the repo that `name` refers to, as a key or as an alias.
    pub fn resolve_repo(&self, name: &str) -> Result<&str> {
        self.repos
            .iter()
            .find(|(key, repo)| *key == name || repo.alias.as_deref() == Some(name))
            .map(|(key, _)| key.as_str())
            .ok_or_else(|| Error::RepoNotFound(name.to_string()))
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
            validated_url_path(&repo.url)?;
            if let Some(b) = &repo.base {
                validate_branch(b)?;
            }
            if let Some(alias) = &repo.alias {
                validate_ident("alias", alias)?;
                self.ensure_alias_is_free(key, alias)?;
            }
        }
        Ok(())
    }

    /// Keys and aliases share one namespace, so a name always points to one repo.
    fn ensure_alias_is_free(&self, owner: &str, alias: &str) -> Result<()> {
        let taken_by = if self.repos.contains_key(alias) {
            Some(alias)
        } else {
            self.repos
                .iter()
                .find(|(key, repo)| *key != owner && repo.alias.as_deref() == Some(alias))
                .map(|(key, _)| key.as_str())
        };
        match taken_by {
            Some(repo) => Err(Error::DuplicateAlias {
                alias: alias.to_string(),
                repo: repo.to_string(),
            }),
            None => Ok(()),
        }
    }
}

/// Derives a repo key from a git url: `git@github.com:org/frontend.git` gives `frontend`.
pub fn repo_key_from_url(url: &str) -> Result<String> {
    let path = validated_url_path(url)?;
    let key = repo_name_from_path(path);
    validate_ident("repo key", key).map_err(|_| Error::NoRepoKey(url.trim().to_string()))?;
    Ok(key.to_string())
}

fn validated_url_path(url: &str) -> Result<&str> {
    url_path(url.trim())
        .filter(|path| !path.trim_matches('/').is_empty())
        .ok_or_else(|| Error::InvalidUrl(url.to_string()))
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
pub(crate) fn validate_branch(value: &str) -> Result<()> {
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
