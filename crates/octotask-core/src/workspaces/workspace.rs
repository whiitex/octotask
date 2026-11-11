use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::cache::{Cache, orphan_caches};
use crate::adapters::origin_ref;
use crate::config::validate_branch;
use crate::{Commit, Error, Git, Home, Project, Repo, Result};

/// What `octok status` shows for one project.
#[derive(Debug, Serialize)]
pub struct Status {
    pub project: String,
    pub repos: Vec<RepoStatus>,
    pub workspaces: Vec<WorkspaceStatus>,
    /// Caches of repos that are no longer in the project file.
    pub orphan_caches: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct RepoStatus {
    pub key: String,
    pub alias: Option<String>,
    pub url: String,
    pub base: String,
    /// Whether the repo has been cloned into the cache.
    pub cached: bool,
    /// Tip of the base branch at the last fetch. `None` if the repo is not cached,
    /// or if the base branch is not on the remote.
    pub commit: Option<Commit>,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceStatus {
    pub name: String,
    pub path: PathBuf,
    pub checkouts: Vec<Checkout>,
}

/// One repo's worktree inside a workspace.
#[derive(Debug, Serialize)]
pub struct Checkout {
    pub repo: String,
    /// `None` on a detached HEAD.
    pub branch: Option<String>,
    pub commit: Commit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOutcome {
    Updated,
    UpToDate,
    /// The branch was never pushed, so there is nothing to pull.
    NoRemoteBranch,
    /// Local and remote both have commits the other lacks: only the user can reconcile them.
    Diverged,
    Detached,
    /// The repo joined the project after the workspace was created.
    NoWorktree,
}

/// Creates a workspace: one worktree per repo, on a new `branch` started from the freshly
/// fetched base branch. Returns the workspace directory.
pub fn create_workspace(home: &Home, project: &Project, branch: &str) -> Result<PathBuf> {
    validate_branch(branch)?;
    if project.repos.is_empty() {
        return Err(Error::NoRepos(project.name.clone()));
    }
    let name = workspace_name(branch);
    let path = home.workspace_dir(&project.name, &name);
    if path.exists() {
        return Err(Error::WorkspaceExists { name, path });
    }
    fetch_all(home, project)?;
    ensure_bases_exist(home, project)?;
    ensure_branch_is_new(home, project, branch)?;
    add_worktrees(home, project, branch, &path)?;
    Ok(path)
}

/// Directory name of the workspace for `branch`: `feat/login` gives `feat-login`.
pub fn workspace_name(branch: &str) -> String {
    branch.replace('/', "-")
}

/// Directory of the existing workspace for `branch`.
pub fn workspace_path(home: &Home, project_name: &str, branch: &str) -> Result<PathBuf> {
    let path = home.workspace_dir(project_name, &workspace_name(branch));
    if path.is_dir() {
        Ok(path)
    } else {
        Err(Error::WorkspaceNotFound(branch.to_string()))
    }
}

/// Project and workspace names of the workspace that contains `dir`, if any.
pub fn locate_workspace(home: &Home, dir: &Path) -> Option<(String, String)> {
    let root = home.workspaces_root().canonicalize().ok()?;
    let dir = dir.canonicalize().ok()?;
    let mut names = dir
        .strip_prefix(root)
        .ok()?
        .components()
        .map(|component| component.as_os_str().to_str().map(str::to_string));
    Some((names.next()??, names.next()??))
}

/// Fetches each repo and fast-forwards its worktree to the remote branch of the same name.
/// Never merges, rebases or resets. A failing repo does not stop the others.
pub fn pull_workspace(
    home: &Home,
    project: &Project,
    workspace: &str,
) -> Result<Vec<(String, Result<PullOutcome>)>> {
    let path = home.workspace_dir(&project.name, workspace);
    if !path.is_dir() {
        return Err(Error::WorkspaceNotFound(workspace.to_string()));
    }
    let report = project
        .repos
        .iter()
        .map(|(key, repo)| {
            let cache = Cache::new(home, &project.name, key);
            (key.clone(), pull_worktree(&cache, repo, &path.join(key)))
        })
        .collect();
    Ok(report)
}

/// Reads the project's repos and workspaces from disk, without touching the network.
pub fn status(home: &Home, project: &Project) -> Result<Status> {
    let repos = project
        .repos
        .iter()
        .map(|(key, repo)| repo_status(home, project, key, repo))
        .collect::<Result<_>>()?;
    let workspaces = workspace_names(home, &project.name)?
        .into_iter()
        .map(|name| workspace_status(home, project, name))
        .collect::<Result<_>>()?;
    Ok(Status {
        project: project.name.clone(),
        repos,
        workspaces,
        orphan_caches: orphan_caches(home, project)?,
    })
}

/// Names of the workspaces that hold a worktree of `repo`.
pub(crate) fn workspaces_with_repo(
    home: &Home,
    project_name: &str,
    repo: &str,
) -> Result<Vec<String>> {
    let mut names = workspace_names(home, project_name)?;
    names.retain(|name| home.workspace_dir(project_name, name).join(repo).is_dir());
    Ok(names)
}

fn fetch_all(home: &Home, project: &Project) -> Result<()> {
    project.repos.iter().try_for_each(|(key, repo)| {
        Cache::new(home, &project.name, key)
            .fetch(&repo.url)
            .map(drop)
    })
}

fn ensure_bases_exist(home: &Home, project: &Project) -> Result<()> {
    for (key, repo) in &project.repos {
        let base = project.effective_base(repo);
        let git = Cache::new(home, &project.name, key).git();
        if !git.origin_branch_exists(base)? {
            return Err(Error::BaseNotFound {
                repo: key.clone(),
                base: base.to_string(),
            });
        }
    }
    Ok(())
}

fn ensure_branch_is_new(home: &Home, project: &Project, branch: &str) -> Result<()> {
    let mut repos = Vec::new();
    for key in project.repos.keys() {
        let git = Cache::new(home, &project.name, key).git();
        if git.origin_branch_exists(branch)? {
            repos.push(format!("{key} (on the remote)"));
        } else if git.branch_exists(branch)? {
            repos.push(format!("{key} (in the cache)"));
        }
    }
    if repos.is_empty() {
        Ok(())
    } else {
        Err(Error::BranchExists {
            branch: branch.to_string(),
            repos,
        })
    }
}

fn add_worktrees(home: &Home, project: &Project, branch: &str, path: &Path) -> Result<()> {
    let mut added: Vec<(Cache, PathBuf)> = Vec::new();
    for (key, repo) in &project.repos {
        let cache = Cache::new(home, &project.name, key);
        let worktree = path.join(key);
        let start_point = origin_ref(project.effective_base(repo));
        let result = cache
            .lock()
            .and_then(|_lock| cache.git().add_worktree(&worktree, branch, &start_point));
        if let Err(error) = result {
            remove_worktrees(&added, branch, path);
            return Err(error);
        }
        added.push((cache, worktree));
    }
    Ok(())
}

/// Best effort: the error that triggered the cleanup is the one worth reporting.
/// The branches were just created at the base commit, so deleting them loses nothing.
fn remove_worktrees(added: &[(Cache, PathBuf)], branch: &str, path: &Path) {
    for (cache, worktree) in added {
        if let Ok(_lock) = cache.lock() {
            let _ = cache.git().remove_worktree(worktree);
            let _ = cache.git().delete_branch(branch);
        }
    }
    let _ = fs::remove_dir(path);
}

fn pull_worktree(cache: &Cache, repo: &Repo, worktree: &Path) -> Result<PullOutcome> {
    if !worktree.is_dir() {
        return Ok(PullOutcome::NoWorktree);
    }
    cache.fetch(&repo.url)?;
    let git = Git::new(worktree);
    let Some(branch) = git.current_branch()? else {
        return Ok(PullOutcome::Detached);
    };
    if !git.origin_branch_exists(&branch)? {
        return Ok(PullOutcome::NoRemoteBranch);
    }
    let remote = origin_ref(&branch);
    if git.is_ancestor(&remote, "HEAD")? {
        return Ok(PullOutcome::UpToDate);
    }
    if !git.is_ancestor("HEAD", &remote)? {
        return Ok(PullOutcome::Diverged);
    }
    git.fast_forward(&remote)?;
    Ok(PullOutcome::Updated)
}

fn repo_status(home: &Home, project: &Project, key: &str, repo: &Repo) -> Result<RepoStatus> {
    let base = project.effective_base(repo);
    let cache = Cache::new(home, &project.name, key);
    let cached = cache.exists();
    let commit = if cached && cache.git().origin_branch_exists(base)? {
        Some(cache.git().last_commit(&origin_ref(base))?)
    } else {
        None
    };
    Ok(RepoStatus {
        key: key.to_string(),
        alias: repo.alias.clone(),
        url: repo.url.clone(),
        base: base.to_string(),
        cached,
        commit,
    })
}

fn workspace_status(home: &Home, project: &Project, name: String) -> Result<WorkspaceStatus> {
    let path = home.workspace_dir(&project.name, &name);
    let checkouts = project
        .repos
        .keys()
        .filter(|key| path.join(key).is_dir())
        .map(|key| checkout(key, &path.join(key)))
        .collect::<Result<_>>()?;
    Ok(WorkspaceStatus {
        name,
        path,
        checkouts,
    })
}

fn checkout(repo: &str, worktree: &Path) -> Result<Checkout> {
    let git = Git::new(worktree);
    Ok(Checkout {
        repo: repo.to_string(),
        branch: git.current_branch()?,
        commit: git.last_commit("HEAD")?,
    })
}

fn workspace_names(home: &Home, project_name: &str) -> Result<Vec<String>> {
    let dir = home.workspaces_dir(project_name);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { path: dir, source }),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| Error::Io {
            path: dir.clone(),
            source,
        })?;
        if entry.path().is_dir()
            && let Ok(name) = entry.file_name().into_string()
        {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}
