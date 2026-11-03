//! Core library for `octok`: project configuration and the on-disk registry.

mod error;
mod home;
mod project;
mod registry;
mod utils;

pub use error::{Error, Result};
pub use home::Home;
pub use project::{Project, Repo, repo_key_from_url};
pub use registry::Registry;
