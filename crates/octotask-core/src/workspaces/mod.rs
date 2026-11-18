//! Repo caches and the workspaces made of their worktrees.

mod cache;
mod project;
mod repos;
mod up;
mod workspace;

pub use project::{remove_project, set_project_location};
pub use repos::{
    NewRepo, RepoChanges, fetch_repos, register_repo, set_workspace_run, unregister_repo,
    update_repo,
};
pub use up::{Running, Stopper, UpEvent, start_workspace};
pub use workspace::{
    Checkout, NewWorkspace, PullOutcome, RepoStatus, Status, WorkspaceStatus, create_workspace,
    locate_workspace, pull_workspace, remove_workspace, status, workspace_name, workspace_path,
};
