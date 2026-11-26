//! Repo caches and the workspaces made of their worktrees.

mod cache;
mod dev;
mod project;
mod repos;
mod up;
mod workspace;

pub use dev::{DevChanges, dev_changes, save_dev_changes};
pub use project::{remove_project, set_project_location};
pub use repos::{
    FetchOutcome, NewRepo, RepoChanges, RepoResult, WorkspaceRepoChanges, check_remote_branch,
    fetch_repos, register_repo, unregister_repo, update_repo, update_workspace_repo,
};
pub use up::{Running, Stopper, UpEvent, start_workspace};
pub use workspace::{
    Checkout, CheckoutOutcome, NewCheckout, NewWorkspace, PullOutcome, RepoStatus, Status,
    WorkspaceStatus, checkout_workspace, create_workspace, locate_workspace, pull_workspace,
    remove_workspace, review_workspace, status, workspace_name, workspace_path,
};
