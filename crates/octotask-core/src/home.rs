use std::path::PathBuf;

use crate::{Error, Result};

/// Root of all `octok` state on disk (`~/.octotask` by default, overridable with `OCTOTASK_HOME`).
#[derive(Debug, Clone)]
pub struct Home {
    root: PathBuf,
}

impl Home {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn from_env() -> Result<Self> {
        match std::env::var_os("OCTOTASK_HOME") {
            Some(root) if !root.is_empty() => Ok(Self::new(root)),
            _ => std::env::home_dir()
                .map(|home| Self::new(home.join(".octotask")))
                .ok_or(Error::NoHome),
        }
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.root.join("projects")
    }

    pub fn project_file(&self, name: &str) -> PathBuf {
        self.projects_dir().join(format!("{name}.yaml"))
    }

    pub fn global_config_file(&self) -> PathBuf {
        self.root.join("config.yaml")
    }
}
