//! Core library for `octok`: project configuration, the on-disk registry and the git and gh adapters.

mod adapters;
mod config;
mod error;
mod home;
mod utils;
mod workspaces;

pub use adapters::{
    Commit, Gh, Git, OriginBranches, Ssh, SshSettings, resolve_location, ssh_config_hosts,
};
pub use config::{
    Location, Project, Registry, Repo, WorkspaceRepo, WorkspaceSettings, repo_key_from_url,
};
pub use error::{Error, Result};
pub use home::Home;
pub use workspaces::{
    Checkout, NewRepo, NewWorkspace, PullOutcome, RepoChanges, RepoStatus, Running, Status,
    Stopper, UpEvent, WorkspaceStatus, create_workspace, fetch_repos, locate_workspace,
    pull_workspace, register_repo, remove_project, remove_workspace, set_project_location,
    set_workspace_run, start_workspace, status, unregister_repo, update_repo, workspace_name,
    workspace_path,
};
