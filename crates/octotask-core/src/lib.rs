//! Core library for `octok`: project configuration, the on-disk registry and the git and gh adapters.

mod adapters;
mod config;
mod error;
mod home;
mod utils;
mod workspaces;

pub use adapters::{
    Commit, Gh, Git, OriginBranches, PullRequest, Ssh, SshSettings, resolve_location,
    ssh_config_hosts,
};
pub use config::{
    Location, Project, Registry, Repo, WorkspaceRepo, WorkspaceSettings, repo_key_from_url,
};
pub use error::{Error, ErrorKind, Result};
pub use home::Home;
pub use workspaces::{
    Checkout, CheckoutOutcome, DevChanges, FetchOutcome, NewCheckout, NewRepo, NewWorkspace,
    PullOutcome, RepoChanges, RepoResult, RepoStatus, Running, Status, Stopper, UpEvent,
    WorkspaceRepoChanges, WorkspaceStatus, check_remote_branch, checkout_workspace,
    create_workspace, dev_changes, fetch_repos, locate_workspace, pull_workspace, register_repo,
    remove_project, remove_workspace, review_workspace, save_dev_changes, set_project_location,
    start_workspace, status, unregister_repo, update_repo, update_workspace_repo, workspace_name,
    workspace_path,
};
