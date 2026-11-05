use std::fs::{self, File};
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::project::validate_ident;
use crate::utils::{
    CONFIG_VERSION, lock_exclusive, parse_versioned, to_yaml, write_atomic, write_new,
};
use crate::{Error, Home, Project, Result};

/// Reads and writes project files and the global config under a [`Home`].
/// Every write holds one lock for the whole home; reads skip it because writes are atomic renames.
#[derive(Debug, Clone)]
pub struct Registry {
    home: Home,
}

impl Registry {
    pub fn new(home: Home) -> Self {
        Self { home }
    }

    pub fn home(&self) -> &Home {
        &self.home
    }

    /// Creates a new project file and makes it the default.
    pub fn init_project(&self, name: &str, base: &str) -> Result<Project> {
        let project = Project::new(name, base)?;
        let _lock = self.lock()?;
        // Read first: an unreadable global config must fail before anything is written.
        self.load_global()?;
        let path = self.home.project_file(name);
        write_new(&path, &to_yaml(&project, &path)?).map_err(|e| match e {
            Error::Io { source, .. } if source.kind() == ErrorKind::AlreadyExists => {
                Error::ProjectExists(name.to_string())
            }
            other => other,
        })?;
        self.save_default_project(name)?;
        Ok(project)
    }

    /// Deletes the project file, clearing the default if it pointed to this project.
    pub fn remove_project(&self, name: &str) -> Result<()> {
        let _lock = self.lock()?;
        self.ensure_exists(name)?;
        let mut config = self.load_global()?;
        // Clear first: a crash in between must not leave a default pointing at nothing.
        if config.default_project.as_deref() == Some(name) {
            config.default_project = None;
            self.save_global(&config)?;
        }
        let path = self.home.project_file(name);
        fs::remove_file(&path).map_err(|source| Error::Io { path, source })
    }

    pub fn load_project(&self, name: &str) -> Result<Project> {
        validate_ident("project name", name)?;
        let path = self.home.project_file(name);
        let raw = read_project_file(&path, name)?;
        let project: Project = parse_versioned(&raw, &path)?;
        ensure_name_matches(&project, name, &path)?;
        project.validate()?;
        Ok(project)
    }

    /// Applies `change` to the stored project, saving it only if the change succeeds.
    pub fn update_project<T>(
        &self,
        name: &str,
        change: impl FnOnce(&mut Project) -> Result<T>,
    ) -> Result<T> {
        let _lock = self.lock()?;
        let mut project = self.load_project(name)?;
        let output = change(&mut project)?;
        self.save_project(&project)?;
        Ok(output)
    }

    pub fn list_projects(&self) -> Result<Vec<String>> {
        let dir = self.home.projects_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(Error::Io { path: dir, source }),
        };
        let mut names = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|source| Error::Io {
                    path: dir.clone(),
                    source,
                })?
                .path();
            if path.extension().is_some_and(|ext| ext == "yaml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && validate_ident("project name", stem).is_ok()
            {
                names.push(stem.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn default_project(&self) -> Result<Option<String>> {
        Ok(self.load_global()?.default_project)
    }

    pub fn set_default_project(&self, name: &str) -> Result<()> {
        let _lock = self.lock()?;
        self.ensure_exists(name)?;
        self.save_default_project(name)
    }

    pub fn resolve_project(&self, explicit: Option<&str>) -> Result<String> {
        let name = match explicit {
            Some(name) => name.to_string(),
            None => self.default_project()?.ok_or(Error::NoDefaultProject)?,
        };
        self.ensure_exists(&name)?;
        Ok(name)
    }

    fn lock(&self) -> Result<File> {
        lock_exclusive(&self.home.registry_lock_file())
    }

    fn save_default_project(&self, name: &str) -> Result<()> {
        let mut config = self.load_global()?;
        config.default_project = Some(name.to_string());
        self.save_global(&config)
    }

    fn save_project(&self, project: &Project) -> Result<()> {
        project.validate()?;
        let path = self.home.project_file(&project.name);
        write_atomic(&path, &to_yaml(project, &path)?)
    }

    fn ensure_exists(&self, name: &str) -> Result<()> {
        validate_ident("project name", name)?;
        if self.home.project_file(name).is_file() {
            Ok(())
        } else {
            Err(Error::ProjectNotFound(name.to_string()))
        }
    }

    fn load_global(&self) -> Result<GlobalConfig> {
        let path = self.home.global_config_file();
        match fs::read_to_string(&path) {
            Ok(raw) => parse_versioned(&raw, &path),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(GlobalConfig::default()),
            Err(source) => Err(Error::Io { path, source }),
        }
    }

    fn save_global(&self, config: &GlobalConfig) -> Result<()> {
        let path = self.home.global_config_file();
        write_atomic(&path, &to_yaml(config, &path)?)
    }
}

fn read_project_file(path: &Path, name: &str) -> Result<String> {
    fs::read_to_string(path).map_err(|source| match source.kind() {
        ErrorKind::NotFound => Error::ProjectNotFound(name.to_string()),
        _ => Error::Io {
            path: path.to_path_buf(),
            source,
        },
    })
}

fn ensure_name_matches(project: &Project, expected: &str, path: &Path) -> Result<()> {
    if project.name == expected {
        return Ok(());
    }
    Err(Error::NameMismatch {
        path: path.to_path_buf(),
        name: project.name.clone(),
        expected: expected.to_string(),
    })
}

/// `~/.octotask/config.yaml`: settings that are not tied to one project.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalConfig {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_project: Option<String>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            default_project: None,
        }
    }
}
