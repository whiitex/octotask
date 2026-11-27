use std::path::Path;

use super::utils::run_app;
use crate::{Location, Result};

/// An editor of the VS Code family, run from its command line launcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Editor(&'static str);

impl Editor {
    pub const ALL: &[Editor] = &[
        Editor("code"),
        Editor("cursor"),
        Editor("windsurf"),
        Editor("codium"),
    ];

    /// The name users type, which is also the binary.
    pub fn name(self) -> &'static str {
        self.0
    }

    pub fn named(name: &str) -> Option<Editor> {
        Self::ALL
            .iter()
            .copied()
            .find(|editor| editor.name() == name)
    }

    /// The command line that opens `path`: through Remote-SSH when `location` is another machine.
    pub fn arguments(self, location: &Location, path: &Path) -> Vec<String> {
        let mut arguments = Vec::new();
        if let Some(host) = location.host() {
            arguments.extend(["--remote".to_string(), format!("ssh-remote+{host}")]);
        }
        arguments.push(path.display().to_string());
        arguments
    }

    /// Opens `path` in this editor, which runs on this machine whatever the location.
    pub fn open(self, location: &Location, path: &Path) -> Result<()> {
        let arguments = self.arguments(location, path);
        let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
        run_app(self.name(), &arguments)
    }
}
