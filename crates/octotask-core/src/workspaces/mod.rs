//! Repo caches and the workspaces made of their worktrees.

mod cache;
mod project;
mod repos;
mod workspace;

pub use project::remove_project;
pub use repos::{NewRepo, RepoChanges, fetch_repos, register_repo, unregister_repo, update_repo};
pub use workspace::{
    Checkout, NewWorkspace, PullOutcome, RepoStatus, Status, WorkspaceStatus, create_workspace,
    locate_workspace, pull_workspace, remove_workspace, status, workspace_name, workspace_path,
};
