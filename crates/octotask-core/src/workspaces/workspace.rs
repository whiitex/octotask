use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::cache::{Cache, orphan_caches};
use crate::adapters::origin_ref;
use crate::config::validate_branch;
use crate::utils::remove_dir_if_present;
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
    /// `None` when the worktree cannot be read.
    pub commit: Option<Commit>,
    /// Why the worktree cannot be read, e.g. its cache is gone.
    pub error: Option<String>,
}

/// What `create_workspace` did for each repo.
#[derive(Debug)]
pub struct NewWorkspace {
    pub path: PathBuf,
    /// Repos that got a new branch from their base.
    pub created: Vec<String>,
    /// Repos whose branch was still in the cache, e.g. after the folder was deleted by hand.
    pub restored: Vec<String>,
    /// Repos whose worktree was already there.
    pub kept: Vec<String>,
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
/// fetched base branch.
///
/// Run again on a workspace that lacks some worktrees, it adds only those: this completes
/// an interrupted creation and brings in the repos registered later.
pub fn create_workspace(home: &Home, project: &Project, branch: &str) -> Result<NewWorkspace> {
    validate_branch(branch)?;
    if project.repos.is_empty() {
        return Err(Error::NoRepos(project.name.clone()));
    }
    let name = workspace_name(branch);
    let path = home.workspace_dir(&project.name, &name);
    if project.repos.keys().all(|key| path.join(key).is_dir()) {
        return Err(Error::WorkspaceExists { name, path });
    }
    fetch_all(home, project)?;
    let plan = plan_worktrees(home, project, branch, &path)?;
    add_worktrees(home, project, branch, &path, &plan)?;
    Ok(new_workspace(path, plan))
}

/// Deletes the workspace of `branch`: its worktrees, its folder and the local branches.
/// Returns the workspace name. Refused when that would lose work, unless `force`.
///
/// Also cleans up what a folder deleted by hand left in the caches.
pub fn remove_workspace(
    home: &Home,
    project: &Project,
    branch: &str,
    force: bool,
) -> Result<String> {
    let name = workspace_name(branch);
    let path = home.workspace_dir(&project.name, &name);
    let removals = plan_removals(home, project, branch, &path)?;
    if !path.is_dir() && removals.iter().all(|removal| removal.branches.is_empty()) {
        return Err(Error::WorkspaceNotFound(branch.to_string()));
    }
    if !force {
        let reasons = work_at_risk(project, &path, &removals)?;
        if !reasons.is_empty() {
            return Err(Error::WorkspaceHasWork { name, reasons });
        }
    }
    remove_dir_if_present(&path)?;
    removals.iter().try_for_each(forget_worktree)?;
    Ok(name)
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

pub(crate) fn workspace_names(home: &Home, project_name: &str) -> Result<Vec<String>> {
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

/// What a repo needs to be part of the workspace.
enum Step {
    Keep,
    /// The branch is in the cache without a worktree: check it out again.
    Restore,
    Create {
        start_point: String,
    },
}

fn plan_worktrees(
    home: &Home,
    project: &Project,
    branch: &str,
    path: &Path,
) -> Result<Vec<(String, Step)>> {
    let mut plan = Vec::new();
    let mut on_remote = Vec::new();
    for (key, repo) in &project.repos {
        let git = Cache::new(home, &project.name, key).git();
        let step = if path.join(key).is_dir() {
            Step::Keep
        } else if git.branch_exists(branch)? {
            Step::Restore
        } else if git.origin_branch_exists(branch)? {
            on_remote.push(key.clone());
            continue;
        } else {
            Step::Create {
                start_point: base_ref(project, key, repo, &git)?,
            }
        };
        plan.push((key.clone(), step));
    }
    if on_remote.is_empty() {
        Ok(plan)
    } else {
        Err(Error::BranchExists {
            branch: branch.to_string(),
            repos: on_remote,
        })
    }
}

fn base_ref(project: &Project, key: &str, repo: &Repo, git: &Git) -> Result<String> {
    let base = project.effective_base(repo);
    if git.origin_branch_exists(base)? {
        Ok(origin_ref(base))
    } else {
        Err(Error::BaseNotFound {
            repo: key.to_string(),
            base: base.to_string(),
        })
    }
}

fn add_worktrees(
    home: &Home,
    project: &Project,
    branch: &str,
    path: &Path,
    plan: &[(String, Step)],
) -> Result<()> {
    let mut added: Vec<(Cache, PathBuf, &Step)> = Vec::new();
    for (key, step) in plan {
        let cache = Cache::new(home, &project.name, key);
        let worktree = path.join(key);
        let result = cache
            .lock()
            .and_then(|_lock| add_worktree(&cache.git(), &worktree, branch, step));
        if let Err(error) = result {
            undo_worktrees(&added, branch, path);
            return Err(error);
        }
        added.push((cache, worktree, step));
    }
    Ok(())
}

fn add_worktree(git: &Git, worktree: &Path, branch: &str, step: &Step) -> Result<()> {
    match step {
        Step::Keep => Ok(()),
        Step::Restore => {
            git.prune_worktrees()?;
            git.checkout_worktree(worktree, branch)
        }
        Step::Create { start_point } => git.add_worktree(worktree, branch, start_point),
    }
}

/// Best effort: the error that triggered the cleanup is the one worth reporting.
/// Only the branches created by this run are deleted: they are still at the base commit.
fn undo_worktrees(added: &[(Cache, PathBuf, &Step)], branch: &str, path: &Path) {
    for (cache, worktree, step) in added {
        if matches!(step, Step::Keep) {
            continue;
        }
        if let Ok(_lock) = cache.lock() {
            let _ = cache.git().remove_worktree(worktree);
            if matches!(step, Step::Create { .. }) {
                let _ = cache.git().delete_branch(branch);
            }
        }
    }
    // Fails, as it should, when the workspace still holds the worktrees it already had.
    let _ = fs::remove_dir(path);
}

fn new_workspace(path: PathBuf, plan: Vec<(String, Step)>) -> NewWorkspace {
    let mut workspace = NewWorkspace {
        path,
        created: Vec::new(),
        restored: Vec::new(),
        kept: Vec::new(),
    };
    for (key, step) in plan {
        match step {
            Step::Keep => workspace.kept.push(key),
            Step::Restore => workspace.restored.push(key),
            Step::Create { .. } => workspace.created.push(key),
        }
    }
    workspace
}

/// What removing a workspace touches in one repo.
struct Removal {
    key: String,
    cache: Cache,
    worktree: Option<PathBuf>,
    /// Local branches to delete: the workspace's, and the one checked out if it differs.
    branches: Vec<String>,
}

fn plan_removals(
    home: &Home,
    project: &Project,
    branch: &str,
    path: &Path,
) -> Result<Vec<Removal>> {
    project
        .repos
        .keys()
        .map(|key| {
            let cache = Cache::new(home, &project.name, key);
            let worktree = Some(path.join(key)).filter(|worktree| worktree.is_dir());
            let mut branches = Vec::new();
            if cache.exists() && cache.git().branch_exists(branch)? {
                branches.push(branch.to_string());
            }
            // A worktree that cannot be read has no branch to add: `force` still removes it.
            let checked_out = worktree
                .as_ref()
                .and_then(|worktree| Git::new(worktree).current_branch().ok().flatten());
            if let Some(checked_out) = checked_out
                && checked_out != branch
            {
                branches.push(checked_out);
            }
            Ok(Removal {
                key: key.clone(),
                cache,
                worktree,
                branches,
            })
        })
        .collect()
}

/// Everything that exists only in this workspace, one line per finding.
fn work_at_risk(project: &Project, path: &Path, removals: &[Removal]) -> Result<Vec<String>> {
    let mut reasons = Vec::new();
    for removal in removals {
        let key = &removal.key;
        if let Some(worktree) = &removal.worktree {
            let git = Git::new(worktree);
            match (git.has_changes(), git.current_branch()) {
                (Ok(has_changes), Ok(branch)) => {
                    if has_changes {
                        reasons.push(format!("{key}: uncommitted changes"));
                    }
                    if branch.is_none() {
                        let unpushed = git.unpushed_commits("HEAD")?;
                        reasons.extend(unpushed_reason(key, unpushed, "a detached HEAD"));
                    }
                }
                _ => reasons.push(format!("{key}: the worktree cannot be read")),
            }
        }
        for branch in &removal.branches {
            let unpushed = removal
                .cache
                .git()
                .unpushed_commits(&format!("refs/heads/{branch}"))?;
            reasons.extend(unpushed_reason(key, unpushed, &format!("`{branch}`")));
        }
    }
    let other_files = other_files(project, path);
    if !other_files.is_empty() {
        reasons.push(format!(
            "other files in the workspace: {}",
            other_files.join(", ")
        ));
    }
    Ok(reasons)
}

fn unpushed_reason(key: &str, unpushed: usize, place: &str) -> Option<String> {
    let commits = match unpushed {
        0 => return None,
        1 => "commit",
        _ => "commits",
    };
    Some(format!("{key}: {unpushed} unpushed {commits} on {place}"))
}

/// Entries of the workspace folder that are not the worktree of a repo.
fn other_files(project: &Project, path: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !project.repos.contains_key(name))
        .collect();
    names.sort();
    names
}

/// Runs after the folder is gone: drops the worktree from the cache, then its branches.
fn forget_worktree(removal: &Removal) -> Result<()> {
    if !removal.cache.exists() {
        return Ok(());
    }
    let _lock = removal.cache.lock()?;
    let git = removal.cache.git();
    git.prune_worktrees()?;
    removal
        .branches
        .iter()
        .try_for_each(|branch| git.delete_branch(branch))
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
        .collect();
    Ok(WorkspaceStatus {
        name,
        path,
        checkouts,
    })
}

/// A worktree that cannot be read is reported, so one broken repo does not hide the others.
fn checkout(repo: &str, worktree: &Path) -> Checkout {
    let git = Git::new(worktree);
    let read = git
        .current_branch()
        .and_then(|branch| Ok((branch, git.last_commit("HEAD")?)));
    let (branch, commit, error) = match read {
        Ok((branch, commit)) => (branch, Some(commit), None),
        Err(error) => (None, None, Some(error.to_string())),
    };
    Checkout {
        repo: repo.to_string(),
        branch,
        commit,
        error,
    }
}
