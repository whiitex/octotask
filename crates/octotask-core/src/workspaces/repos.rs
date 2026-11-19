use super::cache::Cache;
use super::dev::Store;
use super::workspace::{workspace_name, workspaces_with_repo};
use crate::config::validate_branch;
use crate::{Error, Home, Location, OriginBranches, Project, Registry, Result};

/// What `octok repo add` asks for. `key` defaults to the last path segment of the url,
/// `base` and `location` to the project's.
#[derive(Debug, Clone, Copy, Default)]
pub struct NewRepo<'a> {
    pub url: &'a str,
    pub key: Option<&'a str>,
    pub base: Option<&'a str>,
    pub alias: Option<&'a str>,
    pub location: Option<&'a Location>,
}

/// Registers a repo in the project and clones it into the cache.
/// Nothing is registered when the clone fails. Returns the key the repo was registered under.
///
/// Both questions come after the clone, when the remote is known:
/// - `allow_missing_base` is asked, with the repo key and the branch, whether to go on
///   when the base branch is not on the remote;
/// - `choose_base` is asked, with the repo key and the branches of the remote, which base
///   to use when `repo.base` leaves it open. `None` keeps the base of the project.
pub fn register_repo(
    registry: &Registry,
    project_name: &str,
    repo: &NewRepo,
    allow_missing_base: impl FnOnce(&str, &str) -> bool,
    choose_base: impl FnOnce(&str, &OriginBranches) -> Option<String>,
) -> Result<String> {
    // On a throwaway copy: validates the input and resolves the key before the slow clone.
    let mut draft = registry.load_project(project_name)?;
    let key = add_to_project(&mut draft, repo)?;
    let cache = Cache::of_repo(registry.home(), &draft, &key, &draft.repos[&key]);
    let discard = |created: bool| {
        if created {
            let _ = cache.remove();
        }
    };
    let created = cache
        .fetch(repo.url.trim())
        .inspect_err(|_| discard(matches!(cache.exists(), Ok(false))))?;
    let register = || {
        let chosen_base = match repo.base {
            Some(_) => None,
            None => choose_base(&key, &cache.git().origin_branches()?),
        };
        draft.set_repo_base(&key, chosen_base.as_deref().or(repo.base))?;
        let base = draft.effective_base(&draft.repos[&key]);
        if !cache.git().origin_branch_exists(base)? && !allow_missing_base(&key, base) {
            return Err(Error::BaseNotFound {
                repo: key.clone(),
                base: base.to_string(),
            });
        }
        let resolved = NewRepo {
            key: Some(&key),
            base: draft.repos[&key].base.as_deref(),
            ..*repo
        };
        registry.update_project(project_name, |project| add_to_project(project, &resolved))
    };
    register().inspect_err(|_| discard(created))
}

/// What `octok repo set` changes. A field left to `None` is not touched.
#[derive(Debug, Clone, Copy, Default)]
pub struct RepoChanges<'a> {
    /// `Some(None)` removes the alias.
    pub alias: Option<Option<&'a str>>,
    /// `Some(None)` goes back to the project's base.
    pub base: Option<Option<&'a str>>,
    pub url: Option<&'a str>,
    /// `Some(None)` goes back to the project's location.
    pub location: Option<Option<&'a Location>>,
    /// `Some(None)` removes the setup command.
    pub setup: Option<Option<&'a str>>,
    /// `Some(None)` removes the run command.
    pub run: Option<Option<&'a str>>,
}

/// Changes the settings of the repo with this key or alias. Returns its key.
///
/// A new url, base or location is checked against the remote first, so the cache is fetched.
/// `allow_missing_base` is asked as in [`register_repo`].
///
/// A repo moves to another location only while no workspace uses it, because its worktrees
/// would be left behind. Its cache is cloned at the new location and deleted at the old one.
pub fn update_repo(
    registry: &Registry,
    project_name: &str,
    name: &str,
    changes: &RepoChanges,
    allow_missing_base: impl FnOnce(&str, &str) -> bool,
) -> Result<String> {
    let home = registry.home();
    let project = registry.load_project(project_name)?;
    let key = project.resolve_repo(name)?.to_string();
    let old_repo = &project.repos[&key];
    // On a throwaway copy: validates the input before the fetch.
    let mut draft = project.clone();
    apply_changes(&mut draft, &key, changes)?;
    let repo = &draft.repos[&key];
    let old_cache = Cache::of_repo(home, &project, &key, old_repo);
    let cache = Cache::of_repo(home, &draft, &key, repo);
    let moves = project.effective_location(old_repo, None) != draft.effective_location(repo, None);
    if moves {
        refuse_if_in_use(home, &project, &key)?;
    }
    if changes.base.is_some() || changes.url.is_some() || moves {
        check_against_remote(
            &cache,
            &key,
            &repo.url,
            draft.effective_base(repo),
            allow_missing_base,
        )
        .inspect_err(|_| {
            if moves {
                let _ = cache.remove();
            } else if matches!(cache.exists(), Ok(true)) {
                // The project keeps the old url, and so must the worktrees that push through the cache.
                let _ = cache.point_to(&old_repo.url);
            }
        })?;
    }
    registry.update_project(project_name, |project| {
        apply_changes(project, &key, changes)
    })?;
    if moves {
        old_cache.remove()?;
        dev_store(home, &project, &key).remove()?;
    }
    Ok(key)
}

/// Sets the command that starts the repo with this key or alias in the workspace of
/// `branch` only. `None` goes back to the command of the repo. Returns the key of the repo.
pub fn set_workspace_run(
    registry: &Registry,
    project_name: &str,
    name: &str,
    branch: &str,
    command: Option<&str>,
) -> Result<String> {
    validate_branch(branch)?;
    registry.update_project(project_name, |project| {
        let key = project.resolve_repo(name)?.to_string();
        project.set_workspace_run(&workspace_name(branch), &key, command)?;
        Ok(key)
    })
}

/// Unregisters the repo with this key or alias and deletes its cache. Returns its key.
/// Refused while a workspace still uses the repo, because the commits of its worktrees live in the cache.
pub fn unregister_repo(registry: &Registry, project_name: &str, name: &str) -> Result<String> {
    let project = registry.load_project(project_name)?;
    let key = project.resolve_repo(name)?.to_string();
    refuse_if_in_use(registry.home(), &project, &key)?;
    registry.update_project(project_name, |project| project.remove_repo(&key))?;
    Cache::of_repo(registry.home(), &project, &key, &project.repos[&key]).remove()?;
    dev_store(registry.home(), &project, &key).remove()?;
    Ok(key)
}

/// Fetches every repo of the project, cloning the missing caches.
/// A failing repo does not stop the others: each one gets its own result.
pub fn fetch_repos(home: &Home, project: &Project) -> Vec<(String, Result<()>)> {
    project
        .repos
        .iter()
        .map(|(key, repo)| {
            let result = Cache::of_repo(home, project, key, repo)
                .fetch(&repo.url)
                .map(drop);
            (key.clone(), result)
        })
        .collect()
}

/// Fails unless `branch` is a name git accepts and the remote of the repo `key` has it,
/// as of a fetch made now.
pub fn check_remote_branch(home: &Home, project: &Project, key: &str, branch: &str) -> Result<()> {
    validate_branch(branch)?;
    let repo = project
        .repos
        .get(key)
        .ok_or_else(|| Error::RepoNotFound(key.to_string()))?;
    let cache = Cache::of_repo(home, project, key, repo);
    cache.fetch(&repo.url)?;
    if cache.git().origin_branch_exists(branch)? {
        Ok(())
    } else {
        Err(Error::BranchNotOnRemote {
            repo: key.to_string(),
            branch: branch.to_string(),
        })
    }
}

/// The dev changes saved for a repo where the project keeps it.
pub(super) fn dev_store(home: &Home, project: &Project, key: &str) -> Store {
    let location = project.effective_location(&project.repos[key], None);
    Store::new(home, location, &project.name, key)
}

fn refuse_if_in_use(home: &Home, project: &Project, key: &str) -> Result<()> {
    let workspaces = workspaces_with_repo(home, project, key)?;
    if workspaces.is_empty() {
        Ok(())
    } else {
        Err(Error::RepoInUse {
            repo: key.to_string(),
            workspaces,
        })
    }
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
    if let Some(location) = changes.location {
        project.set_repo_location(key, location.cloned())?;
    }
    if let Some(setup) = changes.setup {
        project.set_repo_setup(key, setup)?;
    }
    if let Some(run) = changes.run {
        project.set_repo_run(key, run)?;
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
    if let Some(location) = repo.location {
        project.set_repo_location(&key, Some(location.clone()))?;
    }
    Ok(key)
}
