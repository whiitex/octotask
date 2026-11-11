use std::fs::{self, File};
use std::io::{self, ErrorKind, IsTerminal};
use std::path::PathBuf;

use crate::utils::lock_exclusive;
use crate::{Error, Git, Home, Project, Result};

/// Caches left behind by repos that are no longer in the project file, e.g. after renaming a key by hand.
pub(crate) fn orphan_caches(home: &Home, project: &Project) -> Result<Vec<PathBuf>> {
    let dir = home.caches_dir(&project.name);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(Error::Io { path: dir, source }),
    };
    let mut orphans = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|source| Error::Io {
                path: dir.clone(),
                source,
            })?
            .path();
        let is_cache = path.is_dir() && path.extension().is_some_and(|ext| ext == "git");
        let is_registered = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|key| project.repos.contains_key(key));
        if is_cache && !is_registered {
            orphans.push(path);
        }
    }
    orphans.sort();
    Ok(orphans)
}

/// The bare clone of one repo, shared by every workspace of its project.
pub(crate) struct Cache {
    dir: PathBuf,
    lock_file: PathBuf,
}

impl Cache {
    pub(crate) fn new(home: &Home, project: &str, repo: &str) -> Self {
        Self {
            dir: home.cache_dir(project, repo),
            lock_file: home.cache_lock_file(project, repo),
        }
    }

    pub(crate) fn git(&self) -> Git {
        Git::new(&self.dir)
    }

    pub(crate) fn exists(&self) -> bool {
        self.dir.is_dir()
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

    /// Deletes the cache and its lock file.
    pub(crate) fn remove(&self) -> Result<()> {
        let _lock = self.lock()?;
        match fs::remove_dir_all(&self.dir) {
            Err(source) if source.kind() != ErrorKind::NotFound => {
                return Err(self.io_error(source));
            }
            _ => {}
        }
        // Still held: a process already waiting on it gets the lock next, on a cache that is gone.
        let _ = fs::remove_file(&self.lock_file);
        Ok(())
    }

    fn ensure_locked(&self, url: &str) -> Result<bool> {
        if self.exists() {
            // The url may have changed in the project file since the clone.
            self.git().set_origin_url(url)?;
            return Ok(false);
        }
        self.clone_from(url)?;
        Ok(true)
    }

    /// Clones into a sibling directory first, so a failed clone never leaves a half-made cache.
    fn clone_from(&self, url: &str) -> Result<()> {
        let staging = self
            .dir
            .with_extension(format!("tmp-{}", std::process::id()));
        fs::create_dir_all(&staging).map_err(|source| self.io_error(source))?;
        let git = Git::new(&staging);
        let cloned = git
            .init_bare()
            .and_then(|()| git.add_origin(url))
            .and_then(|()| fetch_whole_history(&git))
            .and_then(|()| fs::rename(&staging, &self.dir).map_err(|source| self.io_error(source)));
        if cloned.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        cloned
    }

    fn io_error(&self, source: io::Error) -> Error {
        Error::Io {
            path: self.dir.clone(),
            source,
        }
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
