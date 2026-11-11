use super::workspace::workspace_names;
use crate::utils::remove_dir_if_present;
use crate::{Error, Registry, Result};

/// Deletes a project together with the caches of its repos.
/// Refused while it has workspaces, because they may hold work that exists nowhere else.
pub fn remove_project(registry: &Registry, name: &str) -> Result<()> {
    let name = registry.resolve_project(Some(name))?;
    let home = registry.home();
    let workspaces = workspace_names(home, &name)?;
    if !workspaces.is_empty() {
        return Err(Error::ProjectInUse {
            project: name,
            workspaces,
        });
    }
    // Caches first: if this fails the project is still there, and `octok fetch` rebuilds them.
    remove_dir_if_present(&home.caches_dir(&name))?;
    remove_dir_if_present(&home.workspaces_dir(&name))?;
    registry.remove_project(&name)
}
