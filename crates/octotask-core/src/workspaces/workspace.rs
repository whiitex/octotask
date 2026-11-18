use std::path::{Path, PathBuf};

use serde::Serialize;

use super::cache::{Cache, orphan_caches};
use crate::adapters::{Files, origin_ref};
use crate::config::validate_branch;
use crate::{Commit, Error, Git, Home, Location, Project, Registry, Repo, Result};

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
    /// `local`, or `<host>:<octok home>` for a repo on another machine.
    pub location: String,
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
    /// The worktree, written `<host>:<path>` when it is on another machine.
    pub path: PathBuf,
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
    /// The worktrees on other machines, by repo, written `<host>:<path>`.
    pub remote_worktrees: Vec<(String, PathBuf)>,
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
///
/// With a `location`, every repo of the workspace lives there, whatever the repos and the
/// project say. The choice is saved in the project until the workspace is removed.
pub fn create_workspace(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    location: Option<&Location>,
) -> Result<NewWorkspace> {
    validate_branch(branch)?;
    let name = workspace_name(branch);
    let Some(location) = location else {
        let project = registry.load_project(project_name)?;
        return add_worktrees_of_project(registry.home(), &project, branch);
    };
    let (project, newly_placed) = place_workspace(registry, project_name, &name, location)?;
    add_worktrees_of_project(registry.home(), &project, branch).inspect_err(|_| {
        if newly_placed {
            let _ = forget_placement(registry, &project, &name);
        }
    })
}

/// Deletes the workspace of `branch`: its worktrees, its folder and the local branches.
/// Returns the workspace name. Refused when that would lose work, unless `force`.
///
/// Also cleans up what a folder deleted by hand left in the caches, and forgets the
/// settings the workspace had of its own.
pub fn remove_workspace(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    force: bool,
) -> Result<String> {
    let project = registry.load_project(project_name)?;
    let name = workspace_name(branch);
    match remove_worktrees_of_project(registry.home(), &project, branch, force) {
        // Only the settings of the workspace were left to forget.
        Err(Error::WorkspaceNotFound(_)) if project.workspaces.contains_key(&name) => {}
        removed => drop(removed?),
    }
    forget_placement(registry, &project, &name)?;
    forget_commands(registry, &project, &name)?;
    Ok(name)
}

/// Saves `location` as the place of every repo of the workspace.
/// Returns the project with it, and whether the workspace was not there already.
fn place_workspace(
    registry: &Registry,
    project_name: &str,
    name: &str,
    location: &Location,
) -> Result<(Project, bool)> {
    let project = registry.load_project(project_name)?;
    let placed_there = project
        .workspaces
        .get(name)
        .is_some_and(|settings| settings.location.as_ref() == Some(location));
    if placed_there {
        return Ok((project, false));
    }
    let folders = workspace_folders(registry.home(), &project, name);
    if any_exists(&folders)? {
        return Err(Error::WorkspaceExists {
            name: name.to_string(),
            path: shown_path(&folders),
        });
    }
    let placed = registry.update_project(project_name, |project| {
        project.set_workspace_location(name, Some(location.clone()))?;
        Ok(project.clone())
    })?;
    Ok((placed, true))
}

/// Forgets the location a workspace had of its own, and deletes the caches cloned there
/// only for it.
fn forget_placement(registry: &Registry, project: &Project, name: &str) -> Result<()> {
    let settings = project.workspaces.get(name);
    let Some(location) = settings.and_then(|settings| settings.location.as_ref()) else {
        return Ok(());
    };
    let remaining = registry.update_project(&project.name, |project| {
        project.set_workspace_location(name, None)?;
        Ok(project.clone())
    })?;
    let used_by_a_workspace = remaining
        .workspaces
        .values()
        .any(|settings| settings.location.as_ref() == Some(location));
    for (key, repo) in &remaining.repos {
        if !used_by_a_workspace && remaining.effective_location(repo, None) != location {
            Cache::new(registry.home(), location, &remaining.name, key).remove()?;
        }
    }
    Ok(())
}

/// Forgets the commands a workspace had of its own.
fn forget_commands(registry: &Registry, project: &Project, name: &str) -> Result<()> {
    let settings = project.workspaces.get(name);
    if settings.is_some_and(|settings| !settings.repos.is_empty()) {
        registry.update_project(&project.name, |project| {
            project.forget_workspace(name);
            Ok(())
        })?;
    }
    Ok(())
}

fn add_worktrees_of_project(home: &Home, project: &Project, branch: &str) -> Result<NewWorkspace> {
    if project.repos.is_empty() {
        return Err(Error::NoRepos(project.name.clone()));
    }
    let name = workspace_name(branch);
    let members = members(home, project, &name);
    let folders = workspace_folders(home, project, &name);
    let path = shown_path(&folders);
    if all_have_worktrees(&members)? {
        return Err(Error::WorkspaceExists { name, path });
    }
    fetch_all(&members)?;
    let plan = plan_worktrees(project, branch, &members)?;
    add_worktrees(branch, &plan, &folders)?;
    Ok(new_workspace(path, plan))
}

fn remove_worktrees_of_project(
    home: &Home,
    project: &Project,
    branch: &str,
    force: bool,
) -> Result<()> {
    let name = workspace_name(branch);
    let members = members(home, project, &name);
    let folders = workspace_folders(home, project, &name);
    let removals = plan_removals(branch, &members)?;
    if !any_exists(&folders)? && removals.iter().all(|removal| removal.branches.is_empty()) {
        return Err(Error::WorkspaceNotFound(branch.to_string()));
    }
    if !force {
        let reasons = work_at_risk(project, &folders, &removals)?;
        if !reasons.is_empty() {
            return Err(Error::WorkspaceHasWork { name, reasons });
        }
    }
    folders
        .iter()
        .try_for_each(|folder| folder.files().remove_dir_all(&folder.path))?;
    removals.iter().try_for_each(forget_worktree)
}

/// Directory name of the workspace for `branch`: `feat/login` gives `feat-login`.
pub fn workspace_name(branch: &str) -> String {
    branch.replace('/', "-")
}

/// Directory of the existing workspace for `branch`.
pub fn workspace_path(home: &Home, project: &Project, branch: &str) -> Result<PathBuf> {
    let folders = workspace_folders(home, project, &workspace_name(branch));
    if any_exists(&folders)? {
        Ok(shown_path(&folders))
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
    let report = members_of_existing(home, project, workspace)?
        .iter()
        .map(|member| (member.key.to_string(), pull_worktree(member)))
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
    let workspaces = workspace_names(home, &project.name, &project.locations())?
        .into_iter()
        .map(|name| workspace_status(home, project, name))
        .collect();
    Ok(Status {
        project: project.name.clone(),
        repos,
        workspaces,
        orphan_caches: orphan_caches(home, project)?,
    })
}

/// Names of the project's workspaces, looked up on every one of `locations`.
pub(crate) fn workspace_names(
    home: &Home,
    project_name: &str,
    locations: &[Location],
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for location in locations {
        let dir = home.at(location).workspaces_dir(project_name);
        let entries = Files::new(location).entries(&dir)?;
        names.extend(
            entries
                .into_iter()
                .filter(|entry| entry.is_dir)
                .map(|entry| entry.name),
        );
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// Names of the workspaces that hold a worktree of `repo`.
pub(crate) fn workspaces_with_repo(
    home: &Home,
    project: &Project,
    repo: &str,
) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for name in workspace_names(home, &project.name, &project.locations())? {
        let has_worktree = members(home, project, &name)
            .iter()
            .filter(|member| member.key == repo)
            .map(Member::has_worktree)
            .next()
            .transpose()?;
        if has_worktree == Some(true) {
            names.push(name);
        }
    }
    Ok(names)
}

/// One repo's place in a workspace: its cache and its worktree, on the repo's machine.
pub(super) struct Member<'a> {
    pub(super) key: &'a str,
    pub(super) repo: &'a Repo,
    pub(super) location: Location,
    cache: Cache,
    pub(super) worktree: PathBuf,
}

impl Member<'_> {
    pub(super) fn has_worktree(&self) -> Result<bool> {
        Files::new(&self.location).is_dir(&self.worktree)
    }

    fn git(&self) -> Git {
        Git::at(&self.location, &self.worktree)
    }

    fn shown_worktree(&self) -> PathBuf {
        self.location.shown(&self.worktree)
    }
}

/// The repos of a workspace, which must exist.
pub(super) fn members_of_existing<'a>(
    home: &Home,
    project: &'a Project,
    workspace: &str,
) -> Result<Vec<Member<'a>>> {
    if !any_exists(&workspace_folders(home, project, workspace))? {
        return Err(Error::WorkspaceNotFound(workspace.to_string()));
    }
    Ok(members(home, project, workspace))
}

fn members<'a>(home: &Home, project: &'a Project, workspace: &str) -> Vec<Member<'a>> {
    project
        .repos
        .iter()
        .map(|(key, repo)| {
            let location = project.effective_location(repo, Some(workspace)).clone();
            let folder = home.at(&location).workspace_dir(&project.name, workspace);
            Member {
                key,
                repo,
                cache: Cache::new(home, &location, &project.name, key),
                worktree: folder.join(key),
                location,
            }
        })
        .collect()
}

/// The folder of a workspace on one machine. A workspace has one per machine that holds its repos.
struct Folder {
    location: Location,
    path: PathBuf,
}

impl Folder {
    fn files(&self) -> Files<'_> {
        Files::new(&self.location)
    }
}

fn workspace_folders(home: &Home, project: &Project, workspace: &str) -> Vec<Folder> {
    let mut locations: Vec<&Location> = Vec::new();
    for repo in project.repos.values() {
        let location = project.effective_location(repo, Some(workspace));
        if !locations.contains(&location) {
            locations.push(location);
        }
    }
    if locations.is_empty() {
        locations.extend(project.location.as_ref());
    }
    if locations.is_empty() {
        locations.push(&Location::Local);
    }
    locations
        .into_iter()
        .map(|location| Folder {
            path: home.at(location).workspace_dir(&project.name, workspace),
            location: location.clone(),
        })
        .collect()
}

/// The path to show for a workspace: its folder on this machine when it has one.
fn shown_path(folders: &[Folder]) -> PathBuf {
    folders
        .iter()
        .find(|folder| folder.location == Location::Local)
        .or(folders.first())
        .map(|folder| folder.location.shown(&folder.path))
        .unwrap_or_default()
}

fn any_exists(folders: &[Folder]) -> Result<bool> {
    for folder in folders {
        if folder.files().is_dir(&folder.path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn all_have_worktrees(members: &[Member]) -> Result<bool> {
    for member in members {
        if !member.has_worktree()? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn fetch_all(members: &[Member]) -> Result<()> {
    members
        .iter()
        .try_for_each(|member| member.cache.fetch(&member.repo.url).map(drop))
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

type Plan<'m, 'a> = Vec<(&'m Member<'a>, Step)>;

fn plan_worktrees<'m, 'a>(
    project: &Project,
    branch: &str,
    members: &'m [Member<'a>],
) -> Result<Plan<'m, 'a>> {
    let mut plan = Vec::new();
    let mut on_remote = Vec::new();
    for member in members {
        let git = member.cache.git();
        let step = if member.has_worktree()? {
            Step::Keep
        } else if git.branch_exists(branch)? {
            Step::Restore
        } else if git.origin_branch_exists(branch)? {
            on_remote.push(member.key.to_string());
            continue;
        } else {
            Step::Create {
                start_point: base_ref(project, member, &git)?,
            }
        };
        plan.push((member, step));
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

fn base_ref(project: &Project, member: &Member, git: &Git) -> Result<String> {
    let base = project.effective_base(member.repo);
    if git.origin_branch_exists(base)? {
        Ok(origin_ref(base))
    } else {
        Err(Error::BaseNotFound {
            repo: member.key.to_string(),
            base: base.to_string(),
        })
    }
}

fn add_worktrees(branch: &str, plan: &Plan, folders: &[Folder]) -> Result<()> {
    for (done, (member, step)) in plan.iter().enumerate() {
        let result = member
            .cache
            .lock()
            .and_then(|_lock| add_worktree(member, branch, step));
        if let Err(error) = result {
            undo_worktrees(&plan[..done], branch, folders);
            return Err(error);
        }
    }
    Ok(())
}

fn add_worktree(member: &Member, branch: &str, step: &Step) -> Result<()> {
    let git = member.cache.git();
    match step {
        Step::Keep => Ok(()),
        Step::Restore => {
            git.prune_worktrees()?;
            git.checkout_worktree(&member.worktree, branch)
        }
        Step::Create { start_point } => git.add_worktree(&member.worktree, branch, start_point),
    }
}

/// Best effort: the error that triggered the cleanup is the one worth reporting.
/// Only the branches created by this run are deleted: they are still at the base commit.
fn undo_worktrees(added: &[(&Member, Step)], branch: &str, folders: &[Folder]) {
    for (member, step) in added {
        if matches!(step, Step::Keep) {
            continue;
        }
        if let Ok(_lock) = member.cache.lock() {
            let _ = member.cache.git().remove_worktree(&member.worktree);
            if matches!(step, Step::Create { .. }) {
                let _ = member.cache.git().delete_branch(branch);
            }
        }
    }
    for folder in folders {
        // Fails, as it should, when the workspace still holds the worktrees it already had.
        let _ = folder.files().remove_empty_dir(&folder.path);
    }
}

fn new_workspace(path: PathBuf, plan: Plan) -> NewWorkspace {
    let mut workspace = NewWorkspace {
        path,
        created: Vec::new(),
        restored: Vec::new(),
        kept: Vec::new(),
        remote_worktrees: Vec::new(),
    };
    for (member, step) in plan {
        let key = member.key.to_string();
        if member.location != Location::Local {
            let worktree = (key.clone(), member.shown_worktree());
            workspace.remote_worktrees.push(worktree);
        }
        match step {
            Step::Keep => workspace.kept.push(key),
            Step::Restore => workspace.restored.push(key),
            Step::Create { .. } => workspace.created.push(key),
        }
    }
    workspace
}

/// What removing a workspace touches in one repo.
struct Removal<'m, 'a> {
    member: &'m Member<'a>,
    has_worktree: bool,
    /// Local branches to delete: the workspace's, and the one checked out if it differs.
    branches: Vec<String>,
}

fn plan_removals<'m, 'a>(branch: &str, members: &'m [Member<'a>]) -> Result<Vec<Removal<'m, 'a>>> {
    members
        .iter()
        .map(|member| {
            let has_worktree = member.has_worktree()?;
            let mut branches = Vec::new();
            if member.cache.exists()? && member.cache.git().branch_exists(branch)? {
                branches.push(branch.to_string());
            }
            // A worktree that cannot be read has no branch to add: `force` still removes it.
            let checked_out = has_worktree
                .then(|| member.git().current_branch().ok().flatten())
                .flatten();
            if let Some(checked_out) = checked_out
                && checked_out != branch
            {
                branches.push(checked_out);
            }
            Ok(Removal {
                member,
                has_worktree,
                branches,
            })
        })
        .collect()
}

/// Everything that exists only in this workspace, one line per finding.
fn work_at_risk(
    project: &Project,
    folders: &[Folder],
    removals: &[Removal],
) -> Result<Vec<String>> {
    let mut reasons = Vec::new();
    for removal in removals {
        let key = removal.member.key;
        if removal.has_worktree {
            let git = removal.member.git();
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
                .member
                .cache
                .git()
                .unpushed_commits(&format!("refs/heads/{branch}"))?;
            reasons.extend(unpushed_reason(key, unpushed, &format!("`{branch}`")));
        }
    }
    let other_files = other_files(project, folders);
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

/// Entries of the workspace folders that are not the worktree of a repo.
fn other_files(project: &Project, folders: &[Folder]) -> Vec<String> {
    let mut names: Vec<String> = folders
        .iter()
        .flat_map(|folder| folder.files().entries(&folder.path).unwrap_or_default())
        .map(|entry| entry.name)
        .filter(|name| !project.repos.contains_key(name))
        .collect();
    names.sort();
    names
}

/// Runs after the folder is gone: drops the worktree from the cache, then its branches.
fn forget_worktree(removal: &Removal) -> Result<()> {
    let cache = &removal.member.cache;
    if !cache.exists()? {
        return Ok(());
    }
    let _lock = cache.lock()?;
    let git = cache.git();
    git.prune_worktrees()?;
    removal
        .branches
        .iter()
        .try_for_each(|branch| git.delete_branch(branch))
}

fn pull_worktree(member: &Member) -> Result<PullOutcome> {
    if !member.has_worktree()? {
        return Ok(PullOutcome::NoWorktree);
    }
    member.cache.fetch(&member.repo.url)?;
    let git = member.git();
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
    let cache = Cache::of_repo(home, project, key, repo);
    let cached = cache.exists()?;
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
        location: project.effective_location(repo, None).to_string(),
        cached,
        commit,
    })
}

fn workspace_status(home: &Home, project: &Project, name: String) -> WorkspaceStatus {
    let checkouts = members(home, project, &name)
        .iter()
        .filter_map(checkout)
        .collect();
    WorkspaceStatus {
        path: shown_path(&workspace_folders(home, project, &name)),
        name,
        checkouts,
    }
}

/// The worktree of a repo, if it has one in the workspace.
/// A worktree that cannot be read is reported, so one broken repo does not hide the others.
fn checkout(member: &Member) -> Option<Checkout> {
    let git = member.git();
    let read = member.has_worktree().and_then(|has_worktree| {
        if !has_worktree {
            return Ok(None);
        }
        Ok(Some((git.current_branch()?, git.last_commit("HEAD")?)))
    });
    let (branch, commit, error) = match read {
        Ok(None) => return None,
        Ok(Some((branch, commit))) => (branch, Some(commit), None),
        Err(error) => (None, None, Some(error.to_string())),
    };
    Some(Checkout {
        repo: member.key.to_string(),
        path: member.shown_worktree(),
        branch,
        commit,
        error,
    })
}
