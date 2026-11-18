//! Wrappers around the system binaries octok drives, so it inherits the user's auth and config.

mod files;
mod gh;
mod git;
mod shell;
mod ssh;
mod ssh_config;
mod utils;

pub(crate) use files::Files;
pub use gh::Gh;
pub use git::{Commit, Git, OriginBranches, origin_ref};
pub(crate) use shell::{Commands, Lifeline, Supervised, supervise};
pub use ssh::{Ssh, SshSettings, resolve_location};
pub use ssh_config::ssh_config_hosts;
