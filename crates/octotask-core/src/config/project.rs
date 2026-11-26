use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::env::check_env;
use super::location::Location;
use crate::utils::CONFIG_VERSION;
use crate::{Error, Result};

static LOCAL: Location = Location::Local;

/// A project: the set of repos that make up one product, plus their default base branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub version: u32,
    pub name: String,
    pub base: String,
    /// Where the repos live unless they say otherwise. `None` is this machine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(default)]
    pub repos: IndexMap<String, Repo>,
    /// Settings of the workspaces that have any, by workspace name.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub workspaces: IndexMap<String, WorkspaceSettings>,
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
    /// Where this repo lives, overriding the project's location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// Shell command that prepares a checkout before its first run, e.g. `npm ci`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup: Option<String>,
    /// Shell command that starts the repo, run in its checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// Environment of `setup` and `run`, by name. A value may read others through templates.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub env: IndexMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSettings {
    /// Where every repo of the workspace lives, whatever the repos and the project say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    /// The branch of the workspace, saved when it reviews branches: some of its repos
    /// then have another one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Settings of single repos in this workspace, by repo key.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub repos: IndexMap<String, WorkspaceRepo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceRepo {
    /// Shell command that starts the repo in this workspace, instead of the repo's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
    /// The branch the repo has in this workspace, instead of the branch of the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Env values of the repo in this workspace, replacing or adding to the repo's own.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub env: IndexMap<String, String>,
}

impl WorkspaceSettings {
    fn is_empty(&self) -> bool {
        self.location.is_none() && self.branch.is_none() && self.repos.is_empty()
    }

    fn drop_repos_without_settings(&mut self) {
        self.repos
            .retain(|_, repo| repo.run.is_some() || repo.branch.is_some() || !repo.env.is_empty());
    }
}

impl Project {
    pub fn new(name: &str, base: &str) -> Result<Self> {
        validate_ident("project name", name)?;
        validate_branch(base)?;
        Ok(Self {
            version: CONFIG_VERSION,
            name: name.to_string(),
            base: base.to_string(),
            location: None,
            repos: IndexMap::new(),
            workspaces: IndexMap::new(),
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
                location: None,
                setup: None,
                run: None,
                env: IndexMap::new(),
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

    pub fn clear_alias(&mut self, key: &str) -> Result<()> {
        self.repo_mut(key)?.alias = None;
        Ok(())
    }

    /// Sets the base branch of one repo. `None` goes back to the project's base.
    pub fn set_repo_base(&mut self, key: &str, base: Option<&str>) -> Result<()> {
        let repo = self.repo_mut(key)?;
        if let Some(base) = base {
            validate_branch(base)?;
        }
        repo.base = base.map(str::to_string);
        Ok(())
    }

    pub fn set_repo_url(&mut self, key: &str, url: &str) -> Result<()> {
        let repo = self.repo_mut(key)?;
        validated_url_path(url)?;
        repo.url = url.trim().to_string();
        Ok(())
    }

    /// Sets the base branch used by the repos that do not override it.
    pub fn set_base(&mut self, base: &str) -> Result<()> {
        validate_branch(base)?;
        self.base = base.to_string();
        Ok(())
    }

    /// Sets where the repos live unless they say otherwise. `None` is this machine.
    pub fn set_location(&mut self, location: Option<Location>) {
        self.location = location;
    }

    /// Sets where one repo lives. `None` goes back to the project's location.
    pub fn set_repo_location(&mut self, key: &str, location: Option<Location>) -> Result<()> {
        self.repo_mut(key)?.location = location;
        Ok(())
    }

    /// Sets where every repo of one workspace lives. `None` goes back to each repo's location.
    pub fn set_workspace_location(
        &mut self,
        workspace: &str,
        location: Option<Location>,
    ) -> Result<()> {
        validate_workspace_name(workspace)?;
        match location {
            Some(location) => {
                let settings = self.workspaces.entry(workspace.to_string()).or_default();
                settings.location = Some(location);
            }
            None => {
                if let Some(settings) = self.workspaces.get_mut(workspace) {
                    settings.location = None;
                }
                self.drop_workspace_without_settings(workspace);
            }
        }
        Ok(())
    }

    /// Sets the command that prepares a checkout of the repo. `None` removes it.
    pub fn set_repo_setup(&mut self, key: &str, command: Option<&str>) -> Result<()> {
        let repo = self.repo_mut(key)?;
        repo.setup = checked_command("setup command", command)?;
        Ok(())
    }

    /// Sets the command that starts the repo. `None` removes it.
    pub fn set_repo_run(&mut self, key: &str, command: Option<&str>) -> Result<()> {
        let repo = self.repo_mut(key)?;
        repo.run = checked_command("run command", command)?;
        Ok(())
    }

    /// Sets the command that starts one repo in one workspace. `None` goes back to the repo's.
    pub fn set_workspace_run(
        &mut self,
        workspace: &str,
        key: &str,
        command: Option<&str>,
    ) -> Result<()> {
        validate_workspace_name(workspace)?;
        if !self.repos.contains_key(key) {
            return Err(Error::RepoNotFound(key.to_string()));
        }
        match checked_command("run command", command)? {
            Some(run) => {
                let settings = self.workspaces.entry(workspace.to_string()).or_default();
                settings.repos.entry(key.to_string()).or_default().run = Some(run);
            }
            None => {
                if let Some(settings) = self.workspaces.get_mut(workspace) {
                    if let Some(repo) = settings.repos.get_mut(key) {
                        repo.run = None;
                    }
                    settings.drop_repos_without_settings();
                }
                self.drop_workspace_without_settings(workspace);
            }
        }
        Ok(())
    }

    /// Sets one env value of the repo. `None` removes it.
    pub fn set_repo_env(&mut self, key: &str, name: &str, value: Option<&str>) -> Result<()> {
        let repo = self.repo_mut(key)?;
        match value {
            Some(value) => {
                checked_env(key, name, value)?;
                repo.env.insert(name.to_string(), value.to_string());
            }
            None => {
                repo.env.shift_remove(name);
            }
        }
        Ok(())
    }

    /// Sets one env value of a repo in one workspace. `None` goes back to the repo's.
    pub fn set_workspace_env(
        &mut self,
        workspace: &str,
        key: &str,
        name: &str,
        value: Option<&str>,
    ) -> Result<()> {
        validate_workspace_name(workspace)?;
        if !self.repos.contains_key(key) {
            return Err(Error::RepoNotFound(key.to_string()));
        }
        match value {
            Some(value) => {
                checked_env(key, name, value)?;
                let settings = self.workspaces.entry(workspace.to_string()).or_default();
                let repo = settings.repos.entry(key.to_string()).or_default();
                repo.env.insert(name.to_string(), value.to_string());
            }
            None => {
                if let Some(settings) = self.workspaces.get_mut(workspace) {
                    if let Some(repo) = settings.repos.get_mut(key) {
                        repo.env.shift_remove(name);
                    }
                    settings.drop_repos_without_settings();
                }
                self.drop_workspace_without_settings(workspace);
            }
        }
        Ok(())
    }

    /// Saves what one workspace reviews: `branch` is its own, and `reviewed` has the
    /// branch of each repo that reviews another one, by repo key.
    pub fn set_review(
        &mut self,
        workspace: &str,
        branch: &str,
        reviewed: &HashMap<String, String>,
    ) -> Result<()> {
        validate_workspace_name(workspace)?;
        validate_branch(branch)?;
        for (key, reviewed_branch) in reviewed {
            if !self.repos.contains_key(key) {
                return Err(Error::RepoNotFound(key.clone()));
            }
            validate_branch(reviewed_branch)?;
        }
        let in_project_order: Vec<(String, String)> = self
            .repos
            .keys()
            .filter_map(|key| Some((key.clone(), reviewed.get(key)?.clone())))
            .collect();
        let settings = self.workspaces.entry(workspace.to_string()).or_default();
        settings.branch = Some(branch.to_string());
        for (key, reviewed_branch) in in_project_order {
            settings.repos.entry(key).or_default().branch = Some(reviewed_branch);
        }
        Ok(())
    }

    /// What one workspace reviews, if it was saved: its own branch, and the branch of
    /// each repo that reviews another one, by repo key.
    pub fn review(&self, workspace: &str) -> Option<(&str, HashMap<String, String>)> {
        let settings = self.workspaces.get(workspace)?;
        let branch = settings.branch.as_deref()?;
        let reviewed = settings
            .repos
            .iter()
            .filter_map(|(key, repo)| Some((key.clone(), repo.branch.clone()?)))
            .collect();
        Some((branch, reviewed))
    }

    /// Forgets what one workspace reviews.
    pub fn clear_review(&mut self, workspace: &str) {
        if let Some(settings) = self.workspaces.get_mut(workspace) {
            settings.branch = None;
            for repo in settings.repos.values_mut() {
                repo.branch = None;
            }
            settings.drop_repos_without_settings();
        }
        self.drop_workspace_without_settings(workspace);
    }

    /// Forgets every setting one workspace has of its own.
    pub fn forget_workspace(&mut self, workspace: &str) {
        self.workspaces.shift_remove(workspace);
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
        let repo = self
            .repos
            .shift_remove(key)
            .ok_or_else(|| Error::RepoNotFound(key.to_string()))?;
        self.workspaces.retain(|_, settings| {
            settings.repos.shift_remove(key);
            !settings.is_empty()
        });
        Ok(repo)
    }

    /// Base branch for a repo: its own override, else the project base.
    pub fn effective_base<'a>(&'a self, repo: &'a Repo) -> &'a str {
        repo.base.as_deref().unwrap_or(&self.base)
    }

    /// Where a repo lives: the override of the workspace, else its own location,
    /// else the project's, else this machine.
    pub fn effective_location<'a>(
        &'a self,
        repo: &'a Repo,
        workspace: Option<&str>,
    ) -> &'a Location {
        workspace
            .and_then(|name| self.workspaces.get(name))
            .and_then(|settings| settings.location.as_ref())
            .or(repo.location.as_ref())
            .or(self.location.as_ref())
            .unwrap_or(&LOCAL)
    }

    /// Command that starts a repo in a workspace: the override of the workspace, else the repo's.
    pub fn effective_run(&self, key: &str, workspace: &str) -> Option<&str> {
        self.workspaces
            .get(workspace)
            .and_then(|settings| settings.repos.get(key))
            .and_then(|repo| repo.run.as_deref())
            .or_else(|| self.repos.get(key)?.run.as_deref())
    }

    /// Raw env values of a repo in a workspace: the workspace's replace or follow the repo's.
    pub fn effective_env(&self, key: &str, workspace: &str) -> IndexMap<&str, &str> {
        let own = self.repos.get(key).into_iter().flat_map(|repo| &repo.env);
        let of_workspace = self
            .workspaces
            .get(workspace)
            .and_then(|settings| settings.repos.get(key))
            .into_iter()
            .flat_map(|repo| &repo.env);
        own.chain(of_workspace)
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect()
    }

    /// Every machine the project can have folders on, each one once, the project's own first.
    pub fn locations(&self) -> Vec<Location> {
        let project = self.location.as_ref().unwrap_or(&LOCAL);
        let repos = self
            .repos
            .values()
            .filter_map(|repo| repo.location.as_ref());
        let workspaces = self
            .workspaces
            .values()
            .filter_map(|settings| settings.location.as_ref());
        let mut locations: Vec<Location> = Vec::new();
        for location in std::iter::once(project).chain(repos).chain(workspaces) {
            if !locations.contains(location) {
                locations.push(location.clone());
            }
        }
        locations
    }

    /// Checks invariants that serde cannot express, e.g. after loading a hand-edited file.
    pub(crate) fn validate(&self) -> Result<()> {
        validate_ident("project name", &self.name)?;
        validate_branch(&self.base)?;
        if let Some(location) = &self.location {
            location.validate()?;
        }
        for (name, settings) in &self.workspaces {
            validate_workspace_name(name)?;
            if let Some(location) = &settings.location {
                location.validate()?;
            }
            if let Some(branch) = &settings.branch {
                validate_branch(branch)?;
            }
            for (key, repo) in &settings.repos {
                if !self.repos.contains_key(key) {
                    return Err(Error::RepoNotFound(key.clone()));
                }
                checked_command("run command", repo.run.as_deref())?;
                if let Some(branch) = &repo.branch {
                    validate_branch(branch)?;
                }
                for (name, value) in &repo.env {
                    checked_env(key, name, value)?;
                }
            }
        }
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
            if let Some(location) = &repo.location {
                location.validate()?;
            }
            checked_command("setup command", repo.setup.as_deref())?;
            checked_command("run command", repo.run.as_deref())?;
            for (name, value) in &repo.env {
                checked_env(key, name, value)?;
            }
        }
        Ok(())
    }

    fn repo_mut(&mut self, key: &str) -> Result<&mut Repo> {
        self.repos
            .get_mut(key)
            .ok_or_else(|| Error::RepoNotFound(key.to_string()))
    }

    fn drop_workspace_without_settings(&mut self, workspace: &str) {
        if self
            .workspaces
            .get(workspace)
            .is_some_and(WorkspaceSettings::is_empty)
        {
            self.workspaces.shift_remove(workspace);
        }
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

/// A command is handed to a shell as it is: only an empty one is surely a mistake.
fn checked_command(kind: &'static str, command: Option<&str>) -> Result<Option<String>> {
    match command {
        Some(command) if command.trim().is_empty() => Err(Error::InvalidName {
            kind,
            value: command.to_string(),
            reason: "must not be empty",
        }),
        command => Ok(command.map(str::to_string)),
    }
}

fn checked_env(repo: &str, name: &str, value: &str) -> Result<()> {
    check_env(name, value).map_err(|reason| Error::InvalidEnv {
        repo: repo.to_string(),
        name: name.to_string(),
        reason,
    })
}

/// A workspace is named after its branch, with `/` replaced: the name is one folder.
fn validate_workspace_name(value: &str) -> Result<()> {
    let reason = if value.is_empty() {
        "must not be empty"
    } else if value.contains('/') {
        "must not contain `/`"
    } else {
        return Ok(());
    };
    Err(Error::InvalidName {
        kind: "workspace name",
        value: value.to_string(),
        reason,
    })
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
    let broken_part =
        |part: &str| part.is_empty() || part.starts_with('.') || part.ends_with(".lock");
    if value.split('/').any(broken_part)
        || value.contains("..")
        || value.contains("@{")
        || value.ends_with('.')
        || value == "@"
        || value == "HEAD"
    {
        return fail("is not a valid git branch name");
    }
    Ok(())
}
