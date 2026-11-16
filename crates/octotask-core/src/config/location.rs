use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const LOCAL: &str = "local";

/// Where the folders of a repo live: its cache and its worktrees.
/// Written `local` or `<host>:<root>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Location {
    /// This machine, under the octok home.
    Local,
    /// A machine reached over ssh. `host` is what `ssh` takes as its destination, so a `Host` of
    /// `~/.ssh/config` or `user@host`. `root` is the octok home there, an absolute path.
    Remote { host: String, root: PathBuf },
}

impl Location {
    pub fn remote(host: &str, root: impl Into<PathBuf>) -> Result<Self> {
        let location = Self::Remote {
            host: host.to_string(),
            root: root.into(),
        };
        location.validate()?;
        Ok(location)
    }

    /// `path` as shown to the user: `host:/path` when it is on another machine.
    pub fn shown(&self, path: &Path) -> PathBuf {
        match self {
            Self::Local => path.to_path_buf(),
            Self::Remote { host, .. } => remote_path(host, path),
        }
    }

    pub fn host(&self) -> Option<&str> {
        match self {
            Self::Local => None,
            Self::Remote { host, .. } => Some(host),
        }
    }

    /// The variants are public, so a value built by hand is checked before it is saved.
    pub(crate) fn validate(&self) -> Result<()> {
        let Self::Remote { host, root } = self else {
            return Ok(());
        };
        let fail = |reason| {
            Err(Error::InvalidLocation {
                value: self.to_string(),
                reason,
            })
        };
        if let Err(reason) = check_host(host) {
            return fail(reason);
        }
        let root = root.to_string_lossy();
        if !root.starts_with('/') {
            return fail("the path on the remote machine must be absolute");
        }
        if root.chars().any(char::is_control) {
            return fail("the path contains a control character");
        }
        Ok(())
    }
}

impl FromStr for Location {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        if text == LOCAL {
            return Ok(Self::Local);
        }
        match text.split_once(':') {
            Some((host, root)) => Self::remote(host, root),
            None => Err(Error::InvalidLocation {
                value: text.to_string(),
                reason: "expected `local` or `<host>:<absolute path>`",
            }),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Local => f.write_str(LOCAL),
            Self::Remote { host, root } => write!(f, "{host}:{}", root.display()),
        }
    }
}

impl TryFrom<String> for Location {
    type Error = Error;

    fn try_from(text: String) -> Result<Self> {
        text.parse()
    }
}

impl From<Location> for String {
    fn from(location: Location) -> Self {
        location.to_string()
    }
}

/// What the user writes for a machine: `<host>` or `<host>:<path>`.
/// Without a path, or with a relative one, the remote machine has to be asked for its home.
pub(crate) fn split_destination(input: &str) -> Result<(&str, Option<&str>)> {
    let (host, path) = match input.split_once(':') {
        Some((host, path)) => (host, Some(path).filter(|path| !path.is_empty())),
        None => (input, None),
    };
    validate_host(host)?;
    Ok((host, path))
}

pub(crate) fn validate_host(host: &str) -> Result<()> {
    check_host(host).map_err(|reason| Error::InvalidLocation {
        value: host.to_string(),
        reason,
    })
}

pub(crate) fn is_local(input: &str) -> bool {
    input == LOCAL
}

pub(crate) fn remote_path(host: &str, path: &Path) -> PathBuf {
    PathBuf::from(format!("{host}:{}", path.display()))
}

/// `host` ends up on the command line of `ssh`: it must not read as an option or as shell syntax.
fn check_host(host: &str) -> std::result::Result<(), &'static str> {
    if host.is_empty() {
        return Err("the host is missing");
    }
    if host.starts_with('-') {
        return Err("the host must not start with `-`");
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@'))
    {
        return Err("the host may only contain letters, digits, `.`, `_`, `-` and `@`");
    }
    Ok(())
}
