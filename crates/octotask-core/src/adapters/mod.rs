//! Wrappers around the system binaries octok drives, so it inherits the user's auth and config.

mod editor;
mod files;
mod gh;
mod git;
mod shell;
mod ssh;
mod ssh_config;
mod utils;

pub use editor::Editor;
pub(crate) use files::Files;
pub use gh::{Gh, PullRequest};
pub use git::{Commit, Git, OriginBranches, origin_ref};
pub(crate) use shell::{Commands, Lifeline, Supervised, run_script, supervise};
pub use ssh::{Ssh, SshSettings, resolve_location};
pub use ssh_config::ssh_config_hosts;
