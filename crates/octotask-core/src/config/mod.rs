//! Projects and the global config: the model and its YAML files under the home.

mod project;
mod registry;

pub(crate) use project::validate_branch;
pub use project::{Project, Repo, repo_key_from_url};
pub use registry::Registry;
