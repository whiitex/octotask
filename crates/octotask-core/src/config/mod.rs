//! Projects and the global config: the model and its YAML files under the home.

mod location;
mod project;
mod registry;

pub use location::Location;
pub(crate) use location::{is_local, remote_path, split_destination, validate_host};
pub(crate) use project::validate_branch;
pub use project::{Project, Repo, WorkspaceSettings, repo_key_from_url};
pub use registry::Registry;
