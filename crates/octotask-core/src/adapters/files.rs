use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

use super::utils::{Place, run_checked, run_query};
use crate::{Error, Location, Result};

/// Directory operations on the machine of one location.
pub(crate) struct Files<'a> {
    location: &'a Location,
}

pub(crate) struct Entry {
    pub(crate) name: String,
    pub(crate) is_dir: bool,
}

impl<'a> Files<'a> {
    pub(crate) fn new(location: &'a Location) -> Self {
        Self { location }
    }

    pub(crate) fn is_dir(&self, path: &Path) -> Result<bool> {
        match self.location {
            Location::Local => Ok(path.is_dir()),
            Location::Remote { .. } => run_query("test", self.place(), &["-d", &text(path)]),
        }
    }

    /// Entries of `dir`. A directory that does not exist has none.
    pub(crate) fn entries(&self, dir: &Path) -> Result<Vec<Entry>> {
        match self.location {
            Location::Local => local_entries(dir),
            Location::Remote { .. } => self.remote_entries(dir),
        }
    }

    pub(crate) fn create_dir_all(&self, path: &Path) -> Result<()> {
        match self.location {
            Location::Local => fs::create_dir_all(path).map_err(|source| io_error(path, source)),
            Location::Remote { .. } => self.run_remote("mkdir", &["-p", "--", &text(path)]),
        }
    }

    /// Deletes a directory tree. A directory that is already gone is not an error.
    pub(crate) fn remove_dir_all(&self, path: &Path) -> Result<()> {
        match self.location {
            Location::Local => match fs::remove_dir_all(path) {
                Err(source) if source.kind() != ErrorKind::NotFound => Err(io_error(path, source)),
                _ => Ok(()),
            },
            Location::Remote { .. } => self.run_remote("rm", &["-rf", "--", &text(path)]),
        }
    }

    /// Deletes a directory, failing if it still has entries.
    pub(crate) fn remove_empty_dir(&self, path: &Path) -> Result<()> {
        match self.location {
            Location::Local => fs::remove_dir(path).map_err(|source| io_error(path, source)),
            Location::Remote { .. } => self.run_remote("rmdir", &["--", &text(path)]),
        }
    }

    pub(crate) fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        match self.location {
            Location::Local => fs::rename(from, to).map_err(|source| io_error(to, source)),
            Location::Remote { .. } => self.run_remote("mv", &["--", &text(from), &text(to)]),
        }
    }

    /// `ls -p` marks the directories with a trailing `/`.
    fn remote_entries(&self, dir: &Path) -> Result<Vec<Entry>> {
        if !self.is_dir(dir)? {
            return Ok(Vec::new());
        }
        let listing = run_checked("ls", self.place(), &["-1Ap", "--", &text(dir)])?;
        let entries = listing
            .lines()
            .map(|line| match line.strip_suffix('/') {
                Some(name) => Entry {
                    name: name.to_string(),
                    is_dir: true,
                },
                None => Entry {
                    name: line.to_string(),
                    is_dir: false,
                },
            })
            .collect();
        Ok(entries)
    }

    fn run_remote(&self, program: &'static str, args: &[&str]) -> Result<()> {
        run_checked(program, self.place(), args).map(drop)
    }

    fn place(&self) -> Place<'_> {
        Place::machine(self.location)
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn local_entries(dir: &Path) -> Result<Vec<Entry>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error(dir, source)),
    };
    entries
        .map(|entry| {
            let entry = entry.map_err(|source| io_error(dir, source))?;
            Ok(Entry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir: entry.path().is_dir(),
            })
        })
        .collect()
}

fn io_error(path: &Path, source: io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        source,
    }
}
