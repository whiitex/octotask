use std::fs::{self, File};
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use crate::adapters::Files;
use crate::utils::lock_exclusive;
use crate::{Git, Home, Location, Project, Repo, Result};

/// Caches left behind by repos that are no longer in the project file, e.g. after renaming a key by hand.
pub(crate) fn orphan_caches(home: &Home, project: &Project) -> Result<Vec<PathBuf>> {
    let mut orphans = Vec::new();
    for location in project.locations() {
        let dir = home.at(&location).caches_dir(&project.name);
        let caches = Files::new(&location)
            .entries(&dir)?
            .into_iter()
            .filter(|entry| entry.is_dir)
            .filter_map(|entry| {
                let key = entry
                    .name
                    .strip_suffix(".git")
                    .filter(|key| !key.is_empty())?;
                (!project.repos.contains_key(key)).then(|| location.shown(&dir.join(&entry.name)))
            });
        orphans.extend(caches);
    }
    orphans.sort();
    Ok(orphans)
}

/// The bare clone of one repo, shared by every workspace of its project.
pub(crate) struct Cache {
    location: Location,
    dir: PathBuf,
    /// On this machine, whatever the location: it serializes the octok processes that run here.
    lock_file: PathBuf,
}

impl Cache {
    /// The cache of a repo where the project keeps it, whatever the workspace.
    pub(crate) fn of_repo(home: &Home, project: &Project, key: &str, repo: &Repo) -> Self {
        let location = project.effective_location(repo, None);
        Self::new(home, location, &project.name, key)
    }

    pub(crate) fn new(home: &Home, location: &Location, project: &str, repo: &str) -> Self {
        Self {
            location: location.clone(),
            dir: home.at(location).cache_dir(project, repo),
            lock_file: home.cache_lock_file(project, repo),
        }
    }

    pub(crate) fn git(&self) -> Git {
        Git::at(&self.location, &self.dir)
    }

    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn exists(&self) -> Result<bool> {
        self.files().is_dir(&self.dir)
    }

    /// Serializes octok processes that write to this cache. Not reentrant.
    pub(crate) fn lock(&self) -> Result<File> {
        lock_exclusive(&self.lock_file)
    }

    /// Brings the cache up to date with `url`, cloning it first if missing.
    /// Returns whether it was created.
    pub(crate) fn fetch(&self, url: &str) -> Result<bool> {
        let _lock = self.lock()?;
        let created = self.ensure_locked(url)?;
        if !created {
            self.git().fetch_origin()?;
        }
        Ok(created)
    }

    /// Sets the url of `origin` without fetching.
    pub(crate) fn point_to(&self, url: &str) -> Result<()> {
        let _lock = self.lock()?;
        self.git().set_origin_url(url)
    }

    /// Deletes the cache and its lock file.
    pub(crate) fn remove(&self) -> Result<()> {
        let _lock = self.lock()?;
        self.files().remove_dir_all(&self.dir)?;
        // Still held: a process already waiting on it gets the lock next, on a cache that is gone.
        let _ = fs::remove_file(&self.lock_file);
        Ok(())
    }

    fn ensure_locked(&self, url: &str) -> Result<bool> {
        if self.exists()? {
            // The url may have changed in the project file since the clone.
            self.git().set_origin_url(url)?;
            // Caches cloned by an older octok lack it.
            self.git().push_new_branches_to_origin()?;
            return Ok(false);
        }
        self.clone_from(url)?;
        Ok(true)
    }

    /// Clones into a sibling directory first, so a failed clone never leaves a half-made cache.
    fn clone_from(&self, url: &str) -> Result<()> {
        self.remove_interrupted_clones()?;
        let staging = self
            .dir
            .with_extension(format!("tmp-{}", std::process::id()));
        self.files().create_dir_all(&staging)?;
        let git = Git::at(&self.location, &staging);
        let cloned = git
            .init_bare()
            .and_then(|()| git.add_origin(url))
            .and_then(|()| git.push_new_branches_to_origin())
            .and_then(|()| fetch_whole_history(&git))
            .and_then(|()| self.files().rename(&staging, &self.dir));
        if cloned.is_err() {
            let _ = self.files().remove_dir_all(&staging);
        }
        cloned
    }

    /// Staging directories of clones that died before the rename, e.g. on Ctrl-C.
    /// The cache lock is held, so none of them belongs to a running clone.
    fn remove_interrupted_clones(&self) -> Result<()> {
        let (Some(parent), Some(stem)) = (self.dir.parent(), self.dir.file_stem()) else {
            return Ok(());
        };
        let prefix = format!("{}.tmp-", stem.to_string_lossy());
        let Ok(entries) = self.files().entries(parent) else {
            return Ok(());
        };
        for entry in entries {
            let is_staging = entry
                .name
                .strip_prefix(&prefix)
                .is_some_and(|pid| !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()));
            if is_staging {
                self.files().remove_dir_all(&parent.join(&entry.name))?;
            }
        }
        Ok(())
    }

    fn files(&self) -> Files<'_> {
        Files::new(&self.location)
    }
}

/// The slow part of a clone: at a terminal the user watches git's progress instead of a silent wait.
fn fetch_whole_history(git: &Git) -> Result<()> {
    if io::stderr().is_terminal() {
        git.fetch_origin_showing_progress()
    } else {
        git.fetch_origin()
    }
}
