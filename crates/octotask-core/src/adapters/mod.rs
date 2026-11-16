//! Wrappers around the system binaries octok drives, so it inherits the user's auth and config.

mod files;
mod gh;
mod git;
mod ssh;
mod utils;

pub(crate) use files::Files;
pub use gh::Gh;
pub use git::{Commit, Git, origin_ref};
pub use ssh::{Ssh, resolve_location};
