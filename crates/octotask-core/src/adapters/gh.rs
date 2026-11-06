use std::path::PathBuf;

use super::utils::run_checked;
use crate::Result;

const PROGRAM: &str = "gh";

/// Runs the system GitHub CLI `gh` inside one directory.
#[derive(Debug, Clone)]
pub struct Gh {
    dir: PathBuf,
}

impl Gh {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Runs `gh <args>` and returns its stdout. Any non-zero exit is an error.
    pub fn run(&self, args: &[&str]) -> Result<String> {
        run_checked(PROGRAM, &self.dir, args)
    }
}
