//! Core library for `octok`: project configuration, the on-disk registry and the git and gh adapters.

mod adapters;
mod error;
mod home;
mod project;
mod registry;
mod utils;

pub use adapters::{Gh, Git};
pub use error::{Error, Result};
pub use home::Home;
pub use project::{Project, Repo, repo_key_from_url};
pub use registry::Registry;
