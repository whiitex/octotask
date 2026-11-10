//! Repo caches and the workspaces made of their worktrees.

mod cache;
mod repos;
mod workspace;

pub use repos::{NewRepo, fetch_repos, register_repo, unregister_repo};
pub use workspace::{
    Checkout, PullOutcome, RepoStatus, Status, WorkspaceStatus, create_workspace, locate_workspace,
    pull_workspace, status, workspace_name, workspace_path,
};
