//! Core library for `octok`: project configuration, the on-disk registry and the git and gh adapters.

mod adapters;
mod config;
mod error;
mod home;
mod utils;
mod workspaces;

pub use adapters::{Commit, Gh, Git};
pub use config::{Project, Registry, Repo, repo_key_from_url};
pub use error::{Error, Result};
pub use home::Home;
pub use workspaces::{
    Checkout, NewRepo, NewWorkspace, PullOutcome, RepoStatus, Status, WorkspaceStatus,
    create_workspace, fetch_repos, locate_workspace, pull_workspace, register_repo, remove_project,
    remove_workspace, status, unregister_repo, workspace_name, workspace_path,
};
