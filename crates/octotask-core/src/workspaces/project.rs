use super::cache::Cache;
use super::workspace::workspace_names;
use crate::adapters::Files;
use crate::{Error, Home, Location, Registry, Result};

/// Deletes a project together with the caches of its repos, on every machine.
/// Refused while it has workspaces, because they may hold work that exists nowhere else:
/// `force` deletes them too.
pub fn remove_project(registry: &Registry, name: &str, force: bool) -> Result<()> {
    let name = registry.resolve_project(Some(name))?;
    let home = registry.home();
    // A project file that cannot be read is still removed: then only this machine is known.
    let locations = registry
        .load_project(&name)
        .map_or_else(|_| vec![Location::Local], |project| project.locations());
    if !force {
        refuse_if_it_has_workspaces(registry, &name, &locations)?;
    }
    // Caches first: if this fails the project is still there, and `octok fetch` rebuilds them.
    for location in &locations {
        let files = Files::new(location);
        let home = home.at(location);
        files.remove_dir_all(&home.caches_dir(&name))?;
        files.remove_dir_all(&home.workspaces_dir(&name))?;
        if location.host().is_some() {
            remove_home_if_empty(&files, &home);
        }
    }
    registry.remove_project(&name)
}

/// Sets where the repos live unless they say otherwise. `None` is this machine.
/// Refused while the project has workspaces, because their worktrees would be left behind.
///
/// The caches of the repos that move are deleted: `octok fetch` clones them at the new location.
pub fn set_project_location(
    registry: &Registry,
    name: &str,
    location: Option<Location>,
) -> Result<()> {
    let home = registry.home();
    let project = registry.load_project(name)?;
    refuse_if_it_has_workspaces(registry, name, &project.locations())?;
    let mut moved = project.clone();
    moved.set_location(location);
    registry.update_project(name, |project| {
        project.set_location(moved.location.clone());
        Ok(())
    })?;
    for (key, repo) in &project.repos {
        let stays = project.effective_location(repo, None) == moved.effective_location(repo, None);
        if !stays {
            Cache::of_repo(home, &project, key, repo).remove()?;
        }
    }
    Ok(())
}

fn refuse_if_it_has_workspaces(
    registry: &Registry,
    name: &str,
    locations: &[Location],
) -> Result<()> {
    let workspaces = workspace_names(registry.home(), name, locations)?;
    if workspaces.is_empty() {
        Ok(())
    } else {
        Err(Error::ProjectInUse {
            project: name.to_string(),
            workspaces,
        })
    }
}

/// Leaves nothing on another machine once the last project is gone from it.
/// `rmdir` fails on a folder that still has entries, which is what keeps it.
fn remove_home_if_empty(files: &Files, home: &Home) {
    for dir in [home.caches_root(), home.workspaces_root(), home.root()] {
        let _ = files.remove_empty_dir(&dir);
    }
}
