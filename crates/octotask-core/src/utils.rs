use std::fs::{self, File};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub(crate) const CONFIG_VERSION: u32 = 1;

/// Reads only the version, so a newer file fails with a clear message
/// instead of a confusing "unknown field" error.
#[derive(Deserialize)]
struct VersionProbe {
    version: u32,
}

pub(crate) fn parse_versioned<T: DeserializeOwned>(raw: &str, path: &Path) -> Result<T> {
    let probe: VersionProbe = parse(raw, path)?;
    if probe.version != CONFIG_VERSION {
        return Err(Error::UnsupportedVersion {
            path: path.to_path_buf(),
            version: probe.version,
        });
    }
    parse(raw, path)
}

pub(crate) fn to_yaml<T: Serialize>(value: &T, path: &Path) -> Result<String> {
    serde_saphyr::to_string(value).map_err(|e| Error::Serialize {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

pub(crate) fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let tmp = write_temp(path, contents)?;
    fs::rename(&tmp, path).map_err(|source| {
        let _ = fs::remove_file(&tmp);
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    })
}

/// Creates `path` atomically, failing with `AlreadyExists` instead of overwriting.
pub(crate) fn write_new(path: &Path, contents: &str) -> Result<()> {
    let tmp = write_temp(path, contents)?;
    let linked = fs::hard_link(&tmp, path);
    let _ = fs::remove_file(&tmp);
    linked.map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Blocks until the exclusive lock on `path` is held, and keeps it until the returned file is dropped.
/// The OS releases it if the process dies. The file is never deleted: two processes could
/// otherwise end up locking different files at the same path.
pub(crate) fn lock_exclusive(path: &Path) -> Result<File> {
    let io = |source| Error::Io {
        path: path.to_path_buf(),
        source,
    };
    let dir = path.parent().expect("config paths always have a parent");
    fs::create_dir_all(dir).map_err(io)?;
    let file = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(io)?;
    file.lock().map_err(io)?;
    Ok(file)
}

fn parse<T: DeserializeOwned>(raw: &str, path: &Path) -> Result<T> {
    serde_saphyr::from_str(raw).map_err(|e| Error::Parse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Writes to a sibling temp file first, so readers never see a half-written file.
fn write_temp(path: &Path, contents: &str) -> Result<PathBuf> {
    let io = |source| Error::Io {
        path: path.to_path_buf(),
        source,
    };
    let dir = path.parent().expect("config paths always have a parent");
    fs::create_dir_all(dir).map_err(io)?;
    let file_name = path
        .file_name()
        .expect("config paths always have a file name");
    let tmp = dir.join(format!(
        ".{}.tmp-{}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    fs::write(&tmp, contents).map_err(io)?;
    Ok(tmp)
}
