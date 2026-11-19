use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::cache::{Cache, orphan_caches};
use super::dev::Store;
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
    /// The branch of the workspace: the one saved for a review, else the one of its first
    /// checkout. `None` when no checkout has a branch.
    pub branch: Option<String>,
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
    /// Whether the remote had `branch` at the last fetch.
    pub on_remote: bool,
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
    /// Repos that got the branch their remote already had.
    pub tracked: Vec<String>,
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
///
/// Every new checkout gets the dev changes saved for its repo. Nothing is created when
/// some of them no longer apply to the commit a checkout starts from, unless
/// `allow_stale_dev`: the files they are for are then left as they are in git.
pub fn create_workspace(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    location: Option<&Location>,
    allow_stale_dev: bool,
) -> Result<NewWorkspace> {
    let wanted = Wanted::NewBranch;
    open_workspace(
        registry,
        project_name,
        branch,
        wanted,
        location,
        allow_stale_dev,
    )
}

/// Creates the workspace of a `branch` that already exists: a repo whose remote has the
/// branch gets it, and follows it; any other repo gets a new `branch` from its base, so
/// the workspace is complete.
///
/// A workspace that has every worktree is returned as it is, without a fetch. One that
/// lacks some gets only those. `location` and `allow_stale_dev` work as in
/// `create_workspace`.
pub fn checkout_workspace(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    location: Option<&Location>,
    allow_stale_dev: bool,
) -> Result<NewWorkspace> {
    let wanted = Wanted::ExistingBranch;
    open_workspace(
        registry,
        project_name,
        branch,
        wanted,
        location,
        allow_stale_dev,
    )
}

/// Creates a workspace to review branches that already exist, one for each repo named in
/// `chosen`, by key: such a repo gets its branch, and follows it. Any other repo gets a
/// new `branch` from its base, and `branch` names the workspace.
///
/// What the workspace reviews is saved in the project until the workspace is removed, so
/// any later command completes it with the same branches. A workspace that has every
/// worktree is returned as it is, and one that lacks some gets only those. Refused when
/// the workspace exists and was made for something else. `location` and `allow_stale_dev`
/// work as in `create_workspace`.
pub fn review_workspace(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    chosen: &HashMap<String, String>,
    location: Option<&Location>,
    allow_stale_dev: bool,
) -> Result<NewWorkspace> {
    if chosen.is_empty() {
        return Err(Error::NothingToReview);
    }
    let project = registry.load_project(project_name)?;
    for (repo, chosen_branch) in chosen {
        if !project.repos.contains_key(repo) {
            return Err(Error::RepoNotFound(repo.clone()));
        }
        validate_branch(chosen_branch)?;
    }
    validate_branch(branch)?;
    let name = workspace_name(branch);
    let newly_saved = save_review(registry, &project, &name, branch, chosen)?;
    let wanted = Wanted::ExistingBranch;
    open_workspace(
        registry,
        project_name,
        branch,
        wanted,
        location,
        allow_stale_dev,
    )
    .inspect_err(|_| {
        if newly_saved {
            let _ = registry.update_project(project_name, |project| {
                project.clear_review(&name);
                Ok(())
            });
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
    forget_settings(registry, &project, &name)?;
    Ok(name)
}

/// What a workspace is opened for.
#[derive(Clone, Copy)]
enum Wanted<'a> {
    /// A branch to create: no remote may have it yet.
    NewBranch,
    /// A branch that some remote already has.
    ExistingBranch,
    /// The branch to review in each repo, by key. The other repos get the one of the
    /// workspace, as with `ExistingBranch`.
    Review(&'a HashMap<String, String>),
}

impl<'a> Wanted<'a> {
    /// The branch chosen for `repo` alone, which its remote must have.
    fn chosen_branch(self, repo: &str) -> Option<&'a str> {
        match self {
            Self::Review(chosen) => chosen.get(repo).map(String::as_str),
            Self::NewBranch | Self::ExistingBranch => None,
        }
    }
}

fn open_workspace(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    wanted: Wanted,
    location: Option<&Location>,
    allow_stale_dev: bool,
) -> Result<NewWorkspace> {
    validate_branch(branch)?;
    let name = workspace_name(branch);
    let home = registry.home();
    let Some(location) = location else {
        let project = registry.load_project(project_name)?;
        return add_worktrees_of_project(home, &project, branch, wanted, allow_stale_dev);
    };
    let (project, newly_placed) = place_workspace(registry, project_name, &name, location)?;
    add_worktrees_of_project(home, &project, branch, wanted, allow_stale_dev).inspect_err(|_| {
        if newly_placed {
            let _ = forget_placement(registry, &project, &name);
        }
    })
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

/// Saves what the workspace `name` reviews. Returns whether it was not saved already.
/// Fails when the workspace exists and was made for something else.
fn save_review(
    registry: &Registry,
    project: &Project,
    name: &str,
    branch: &str,
    chosen: &HashMap<String, String>,
) -> Result<bool> {
    if let Some((saved_branch, saved)) = project.review(name) {
        if saved_branch == branch && &saved == chosen {
            return Ok(false);
        }
        let reviewed = project
            .repos
            .keys()
            .filter_map(|key| Some(format!("{key}={}", saved.get(key)?)))
            .collect();
        return Err(Error::ReviewsOtherBranches {
            name: name.to_string(),
            reviewed,
        });
    }
    let folders = workspace_folders(registry.home(), project, name);
    if any_exists(&folders)? {
        return Err(Error::WorkspaceExists {
            name: name.to_string(),
            path: shown_path(&folders),
        });
    }
    registry.update_project(&project.name, |project| {
        project.set_review(name, branch, chosen)
    })?;
    Ok(true)
}

/// Forgets the commands a workspace had of its own, and what it reviewed.
fn forget_settings(registry: &Registry, project: &Project, name: &str) -> Result<()> {
    let settings = project.workspaces.get(name);
    if settings.is_some_and(|settings| settings.branch.is_some() || !settings.repos.is_empty()) {
        registry.update_project(&project.name, |project| {
            project.forget_workspace(name);
            Ok(())
        })?;
    }
    Ok(())
}

fn add_worktrees_of_project(
    home: &Home,
    project: &Project,
    branch: &str,
    wanted: Wanted,
    allow_stale_dev: bool,
) -> Result<NewWorkspace> {
    if project.repos.is_empty() {
        return Err(Error::NoRepos(project.name.clone()));
    }
    let name = workspace_name(branch);
    let members = members(home, project, &name);
    let folders = workspace_folders(home, project, &name);
    let path = shown_path(&folders);
    refuse_workspace_of_another_branch(branch, &name, &members)?;
    let review = project.review(&name);
    if let Some((saved_branch, _)) = &review
        && *saved_branch != branch
    {
        return Err(Error::WorkspaceOfAnotherBranch {
            name,
            branch: saved_branch.to_string(),
        });
    }
    if all_have_worktrees(&members)? {
        return match wanted {
            Wanted::NewBranch => Err(Error::WorkspaceExists { name, path }),
            Wanted::ExistingBranch | Wanted::Review(_) => {
                Ok(new_workspace(path, keep_all(branch, &members)))
            }
        };
    }
    fetch_all(&members)?;
    // A workspace that reviews branches is completed with those, whatever asks for it.
    let wanted = match &review {
        Some((_, reviewed)) => Wanted::Review(reviewed),
        None => wanted,
    };
    let plan = plan_worktrees(project, branch, wanted, &members)?;
    prepare_dev_changes(&plan, allow_stale_dev)?;
    add_worktrees(&plan, &folders)?;
    Ok(new_workspace(path, plan))
}

/// `feat/login` and `feat-login` give the same folder: only one of them has the workspace.
fn refuse_workspace_of_another_branch(branch: &str, name: &str, members: &[Member]) -> Result<()> {
    for member in members {
        if !member.has_worktree()? {
            continue;
        }
        // A worktree that cannot be read is reported by `octok status`, not here.
        let checked_out = member.git().current_branch().ok().flatten();
        if let Some(other) = checked_out
            && other != branch
            && workspace_name(&other) == name
        {
            return Err(Error::WorkspaceOfAnotherBranch {
                name: name.to_string(),
                branch: other,
            });
        }
    }
    Ok(())
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
    let removals = plan_removals(project, branch, &members)?;
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
    /// The dev changes saved for the repo on the machine of this checkout.
    pub(super) dev: Store,
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

/// Keys of the repos named by key or alias. `None` when no repo is named: all of them.
pub(super) fn chosen_keys<'a>(
    project: &'a Project,
    names: &[&str],
) -> Result<Option<Vec<&'a str>>> {
    if names.is_empty() {
        return Ok(None);
    }
    let keys: Result<Vec<&str>> = names
        .iter()
        .map(|name| project.resolve_repo(name))
        .collect();
    keys.map(Some)
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
                dev: Store::new(home, &location, &project.name, key),
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
    /// The branch is on the remote: create the local one from it.
    Track,
}

/// One repo of the workspace, the branch it gets there and what that takes.
struct Planned<'m, 'a> {
    member: &'m Member<'a>,
    branch: &'m str,
    step: Step,
}

type Plan<'m, 'a> = Vec<Planned<'m, 'a>>;

fn keep_all<'m, 'a>(branch: &'m str, members: &'m [Member<'a>]) -> Plan<'m, 'a> {
    let keep = |member| Planned {
        member,
        branch,
        step: Step::Keep,
    };
    members.iter().map(keep).collect()
}

fn plan_worktrees<'m, 'a>(
    project: &Project,
    branch: &'m str,
    wanted: Wanted<'m>,
    members: &'m [Member<'a>],
) -> Result<Plan<'m, 'a>> {
    let mut plan = Vec::new();
    for member in members {
        let chosen = wanted.chosen_branch(member.key);
        let branch = chosen.unwrap_or(branch);
        let step = plan_step(project, member, branch, chosen.is_some())?;
        plan.push(Planned {
            member,
            branch,
            step,
        });
    }
    match wanted {
        Wanted::NewBranch => refuse_branch_on_a_remote(branch, &plan)?,
        Wanted::ExistingBranch => refuse_branch_on_no_remote(branch, &plan)?,
        Wanted::Review(_) => {}
    }
    Ok(plan)
}

/// `must_exist` refuses to start `branch` from the base when the remote does not have it.
fn plan_step(project: &Project, member: &Member, branch: &str, must_exist: bool) -> Result<Step> {
    let git = member.cache.git();
    if member.has_worktree()? {
        return Ok(Step::Keep);
    }
    if git.branch_exists(branch)? {
        return match git.worktree_of_branch(branch)? {
            None => Ok(Step::Restore),
            Some(worktree) => Err(Error::BranchInUse {
                repo: member.key.to_string(),
                branch: branch.to_string(),
                path: member.location.shown(&worktree),
            }),
        };
    }
    if git.origin_branch_exists(branch)? {
        return Ok(Step::Track);
    }
    if must_exist {
        return Err(Error::BranchNotOnRemote {
            repo: member.key.to_string(),
            branch: branch.to_string(),
        });
    }
    let start_point = base_ref(project, member, &git)?;
    Ok(Step::Create { start_point })
}

fn refuse_branch_on_a_remote(branch: &str, plan: &Plan) -> Result<()> {
    let on_remote: Vec<String> = plan
        .iter()
        .filter(|planned| matches!(planned.step, Step::Track))
        .map(|planned| planned.member.key.to_string())
        .collect();
    if on_remote.is_empty() {
        Ok(())
    } else {
        Err(Error::BranchExists {
            branch: branch.to_string(),
            repos: on_remote,
        })
    }
}

/// A workspace that already has the branch somewhere may be completed from the base.
fn refuse_branch_on_no_remote(branch: &str, plan: &Plan) -> Result<()> {
    let nowhere = plan
        .iter()
        .all(|planned| matches!(planned.step, Step::Create { .. }));
    if nowhere {
        Err(Error::BranchNotFound(branch.to_string()))
    } else {
        Ok(())
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

/// Makes git keep the saved dev changes in the checkouts about to be added.
/// Fails when some of them no longer apply to the commit a checkout starts from, unless
/// `allow_stale`.
fn prepare_dev_changes(plan: &Plan, allow_stale: bool) -> Result<()> {
    for Planned {
        member,
        branch,
        step,
    } in plan
    {
        let rev = match step {
            Step::Keep => continue,
            Step::Restore => format!("refs/heads/{branch}"),
            Step::Create { start_point } => start_point.clone(),
            Step::Track => origin_ref(branch),
        };
        let stale = member.dev.prepare(member.cache.dir(), &rev)?;
        if !stale.is_empty() && !allow_stale {
            return Err(Error::StaleDevChanges {
                repo: member.key.to_string(),
                files: stale,
            });
        }
    }
    Ok(())
}

fn add_worktrees(plan: &Plan, folders: &[Folder]) -> Result<()> {
    for (done, planned) in plan.iter().enumerate() {
        let result = planned
            .member
            .cache
            .lock()
            .and_then(|_lock| add_worktree(planned));
        if let Err(error) = result {
            // The one that failed may be half done.
            undo_worktrees(&plan[..=done], folders);
            return Err(error);
        }
    }
    Ok(())
}

fn add_worktree(planned: &Planned) -> Result<()> {
    let Planned {
        member,
        branch,
        step,
    } = planned;
    let git = member.cache.git();
    if matches!(step, Step::Keep) {
        return Ok(());
    }
    // A folder deleted by hand is still a worktree for git, which then refuses its path.
    git.prune_worktrees()?;
    match step {
        Step::Keep => {}
        Step::Restore => git.checkout_worktree(&member.worktree, branch)?,
        Step::Create { start_point } => {
            git.add_worktree(&member.worktree, branch, start_point)?;
        }
        Step::Track => git.add_tracking_worktree(&member.worktree, branch)?,
    }
    member.dev.copy_new_files(&member.worktree)
}

/// Best effort: the error that triggered the cleanup is the one worth reporting.
/// Only the branches created by this run are deleted: they are still at a commit of the
/// remote.
fn undo_worktrees(added: &[Planned], folders: &[Folder]) {
    for Planned {
        member,
        branch,
        step,
    } in added
    {
        if matches!(step, Step::Keep) {
            continue;
        }
        if let Ok(_lock) = member.cache.lock() {
            let _ = member.cache.git().remove_worktree(&member.worktree);
            if matches!(step, Step::Create { .. } | Step::Track) {
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
        tracked: Vec::new(),
        restored: Vec::new(),
        kept: Vec::new(),
        remote_worktrees: Vec::new(),
    };
    for Planned { member, step, .. } in plan {
        let key = member.key.to_string();
        if member.location != Location::Local {
            let worktree = (key.clone(), member.shown_worktree());
            workspace.remote_worktrees.push(worktree);
        }
        match step {
            Step::Keep => workspace.kept.push(key),
            Step::Restore => workspace.restored.push(key),
            Step::Create { .. } => workspace.created.push(key),
            Step::Track => workspace.tracked.push(key),
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

fn plan_removals<'m, 'a>(
    project: &Project,
    branch: &str,
    members: &'m [Member<'a>],
) -> Result<Vec<Removal<'m, 'a>>> {
    let review = project.review(&workspace_name(branch));
    members
        .iter()
        .map(|member| {
            let has_worktree = member.has_worktree()?;
            // A worktree that cannot be read has no branch to add: `force` still removes it.
            let checked_out = has_worktree
                .then(|| member.git().current_branch().ok().flatten())
                .flatten();
            let reviewed = review
                .as_ref()
                .and_then(|(_, reviewed)| reviewed.get(member.key));
            let saved_branch = review.as_ref().map(|(saved_branch, _)| *saved_branch);
            let mut branches: Vec<String> = Vec::new();
            let candidates = [
                Some(branch),
                saved_branch,
                reviewed.map(String::as_str),
                checked_out.as_deref(),
            ];
            for candidate in candidates.into_iter().flatten() {
                let known = branches.iter().any(|branch| branch == candidate);
                if !known && belongs_to_removal(member, candidate, checked_out.as_deref())? {
                    branches.push(candidate.to_string());
                }
            }
            Ok(Removal {
                member,
                has_worktree,
                branches,
            })
        })
        .collect()
}

/// Whether the local `branch` goes away with the worktree of `member`: it does unless
/// another workspace has it checked out.
fn belongs_to_removal(member: &Member, branch: &str, checked_out: Option<&str>) -> Result<bool> {
    if !member.cache.exists()? {
        return Ok(false);
    }
    let git = member.cache.git();
    if !git.branch_exists(branch)? {
        return Ok(false);
    }
    Ok(checked_out == Some(branch) || git.worktree_of_branch(branch)?.is_none())
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
    // A branch that was new here and was pushed from elsewhere does not follow it yet.
    git.follow_origin_branch(&branch)?;
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
    let checkouts: Vec<Checkout> = members(home, project, &name)
        .iter()
        .filter_map(checkout)
        .collect();
    let saved_branch = project.review(&name).map(|(branch, _)| branch.to_string());
    let first_branch = || {
        checkouts
            .iter()
            .find_map(|checkout| checkout.branch.clone())
    };
    WorkspaceStatus {
        branch: saved_branch.or_else(first_branch),
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
        let branch = git.current_branch()?;
        let on_remote = match &branch {
            Some(branch) => git.origin_branch_exists(branch)?,
            None => false,
        };
        Ok(Some((branch, on_remote, git.last_commit("HEAD")?)))
    });
    let (branch, on_remote, commit, error) = match read {
        Ok(None) => return None,
        Ok(Some((branch, on_remote, commit))) => (branch, on_remote, Some(commit), None),
        Err(error) => (None, false, None, Some(error.to_string())),
    };
    Some(Checkout {
        repo: member.key.to_string(),
        path: member.shown_worktree(),
        branch,
        on_remote,
        commit,
        error,
    })
}
