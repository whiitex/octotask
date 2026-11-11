use super::cache::Cache;
use super::workspace::workspaces_with_repo;
use crate::{Error, Home, Project, Registry, Result};

/// What `octok repo add` asks for. `key` defaults to the last path segment of the url,
/// `base` to the project's base branch.
#[derive(Debug, Clone, Copy, Default)]
pub struct NewRepo<'a> {
    pub url: &'a str,
    pub key: Option<&'a str>,
    pub base: Option<&'a str>,
    pub alias: Option<&'a str>,
}

/// Registers a repo in the project and clones it into the cache.
/// Nothing is registered when the clone fails. Returns the key the repo was registered under.
///
/// `allow_missing_base` is asked, with the repo key and the branch, whether to go on
/// when the base branch is not on the remote.
pub fn register_repo(
    registry: &Registry,
    project_name: &str,
    repo: &NewRepo,
    allow_missing_base: impl FnOnce(&str, &str) -> bool,
) -> Result<String> {
    // On a throwaway copy: validates the input and resolves the key before the slow clone.
    let mut draft = registry.load_project(project_name)?;
    let key = add_to_project(&mut draft, repo)?;
    let base = draft.effective_base(&draft.repos[&key]);
    let cache = Cache::new(registry.home(), project_name, &key);
    let discard = |created: bool| {
        if created {
            let _ = cache.remove();
        }
    };
    let created = cache
        .fetch(repo.url.trim())
        .inspect_err(|_| discard(!cache.exists()))?;
    if !cache.git().origin_branch_exists(base)? && !allow_missing_base(&key, base) {
        discard(created);
        return Err(Error::BaseNotFound {
            repo: key,
            base: base.to_string(),
        });
    }
    let with_resolved_key = NewRepo {
        key: Some(&key),
        ..*repo
    };
    registry
        .update_project(project_name, |project| {
            add_to_project(project, &with_resolved_key)
        })
        .inspect_err(|_| discard(created))
}

/// What `octok repo set` changes. A field left to `None` is not touched.
#[derive(Debug, Clone, Copy, Default)]
pub struct RepoChanges<'a> {
    /// `Some(None)` removes the alias.
    pub alias: Option<Option<&'a str>>,
    /// `Some(None)` goes back to the project's base.
    pub base: Option<Option<&'a str>>,
    pub url: Option<&'a str>,
}

/// Changes the settings of the repo with this key or alias. Returns its key.
///
/// A new url or base is checked against the remote first, so the cache is fetched.
/// `allow_missing_base` is asked as in [`register_repo`].
pub fn update_repo(
    registry: &Registry,
    project_name: &str,
    name: &str,
    changes: &RepoChanges,
    allow_missing_base: impl FnOnce(&str, &str) -> bool,
) -> Result<String> {
    // On a throwaway copy: validates the input before the fetch.
    let mut draft = registry.load_project(project_name)?;
    let key = draft.resolve_repo(name)?.to_string();
    let old_url = draft.repos[&key].url.clone();
    apply_changes(&mut draft, &key, changes)?;
    if changes.base.is_some() || changes.url.is_some() {
        let repo = &draft.repos[&key];
        let cache = Cache::new(registry.home(), project_name, &key);
        check_against_remote(
            &cache,
            &key,
            &repo.url,
            draft.effective_base(repo),
            allow_missing_base,
        )
        // The project keeps the old url, and so must the worktrees that push through the cache.
        .inspect_err(|_| {
            if cache.exists() {
                let _ = cache.point_to(&old_url);
            }
        })?;
    }
    registry.update_project(project_name, |project| {
        apply_changes(project, &key, changes)
    })?;
    Ok(key)
}

/// Unregisters the repo with this key or alias and deletes its cache. Returns its key.
/// Refused while a workspace still uses the repo, because the commits of its worktrees live in the cache.
pub fn unregister_repo(registry: &Registry, project_name: &str, name: &str) -> Result<String> {
    let project = registry.load_project(project_name)?;
    let key = project.resolve_repo(name)?.to_string();
    let workspaces = workspaces_with_repo(registry.home(), project_name, &key)?;
    if !workspaces.is_empty() {
        return Err(Error::RepoInUse {
            repo: key,
            workspaces,
        });
    }
    registry.update_project(project_name, |project| project.remove_repo(&key))?;
    Cache::new(registry.home(), project_name, &key).remove()?;
    Ok(key)
}

/// Fetches every repo of the project, cloning the missing caches.
/// A failing repo does not stop the others: each one gets its own result.
pub fn fetch_repos(home: &Home, project: &Project) -> Vec<(String, Result<()>)> {
    project
        .repos
        .iter()
        .map(|(key, repo)| {
            let result = Cache::new(home, &project.name, key)
                .fetch(&repo.url)
                .map(drop);
            (key.clone(), result)
        })
        .collect()
}

fn apply_changes(project: &mut Project, key: &str, changes: &RepoChanges) -> Result<()> {
    match changes.alias {
        Some(Some(alias)) => project.set_alias(key, alias)?,
        Some(None) => project.clear_alias(key)?,
        None => {}
    }
    if let Some(base) = changes.base {
        project.set_repo_base(key, base)?;
    }
    if let Some(url) = changes.url {
        project.set_repo_url(key, url)?;
    }
    Ok(())
}

fn check_against_remote(
    cache: &Cache,
    key: &str,
    url: &str,
    base: &str,
    allow_missing_base: impl FnOnce(&str, &str) -> bool,
) -> Result<()> {
    cache.fetch(url)?;
    if cache.git().origin_branch_exists(base)? || allow_missing_base(key, base) {
        Ok(())
    } else {
        Err(Error::BaseNotFound {
            repo: key.to_string(),
            base: base.to_string(),
        })
    }
}

fn add_to_project(project: &mut Project, repo: &NewRepo) -> Result<String> {
    let key = project.add_repo(repo.url, repo.key, repo.base)?;
    if let Some(alias) = repo.alias {
        project.set_alias(&key, alias)?;
    }
    Ok(key)
}
