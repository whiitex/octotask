use std::path::PathBuf;

use super::utils::{failure, run_captured, run_checked};
use crate::Result;

const PROGRAM: &str = "git";

/// Runs the system `git` inside one directory.
#[derive(Debug, Clone)]
pub struct Git {
    dir: PathBuf,
}

impl Git {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Runs `git <args>` and returns its stdout. Any non-zero exit is an error.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        run_checked(PROGRAM, &self.dir, args)
    }

    /// Runs a yes/no query such as `show-ref --verify --quiet`:
    /// exit 0 is `true`, exit 1 is `false`, anything else is an error.
    pub fn check(&self, args: &[&str]) -> Result<bool> {
        let output = run_captured(PROGRAM, &self.dir, args)?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(failure(PROGRAM, &self.dir, args, &output)),
        }
    }
}
