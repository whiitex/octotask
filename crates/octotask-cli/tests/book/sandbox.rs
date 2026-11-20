//! A fresh machine for one page: its own home, git config and fixture remotes, no network.

use std::env;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use crate::remotes;
use crate::shell::Shell;

/// "Now" for git, so relative dates such as `2 days ago` never change: 2026-03-15 12:00 UTC.
pub const NOW: u64 = 1_773_576_000;

/// The date of a commit made by an example: 10 minutes before `NOW`.
const EXAMPLE_COMMIT_AGE: u64 = 600;

/// Where a reader of the book has their home.
const READER_HOME: &str = "/Users/you";

/// The fixture remotes are under this folder, at the path of their GitHub url.
const GITHUB: &str = "git@github.com:";

pub struct Sandbox {
    /// Deleted with the sandbox.
    _dir: TempDir,
    root: PathBuf,
    /// The sandbox folders and the text a reader sees in their place, most specific first.
    redactions: Vec<(String, String)>,
}

impl Sandbox {
    pub fn new() -> Self {
        // Under `/tmp`, so the folders are as short on macOS as on Linux: the project file folds
        // a location that is too long, and the examples would differ between the two.
        let dir = TempDir::with_prefix_in("octok-", "/tmp").unwrap();
        let root = dir.path().canonicalize().unwrap();
        let sandbox = Self {
            redactions: redactions(&root, dir.path()),
            _dir: dir,
            root,
        };
        sandbox.create_folders();
        sandbox.write_git_config();
        sandbox.install_programs();
        sandbox.describe_other_machine();
        remotes::create(&sandbox);
        sandbox
    }

    /// A shell in the home of the sandbox.
    pub fn shell(&self) -> Shell {
        Shell::start(self.command("sh"))
    }

    /// `program` with the environment of the sandbox only, started in its home.
    pub fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .env_clear()
            .envs(self.environment())
            .current_dir(self.home());
        command
    }

    /// `output` as a reader sees it: sandbox folders become the ones of a real machine.
    pub fn redact(&self, output: &str) -> String {
        self.redactions
            .iter()
            .fold(output.to_string(), |text, (from, to)| {
                text.replace(from, to)
            })
    }

    /// The bare repository behind `git@github.com:<path>`.
    pub fn remote_dir(&self, path: &str) -> PathBuf {
        self.remotes_dir().join(path)
    }

    /// A scratch folder that no page sees.
    pub fn work_dir(&self) -> PathBuf {
        self.root.join("work")
    }

    /// The folder that the fake `gh` reads the pull requests from: `<repo>/<number>.json`.
    pub fn pull_requests_dir(&self) -> PathBuf {
        self.fakes_dir().join("pull-requests")
    }

    fn create_folders(&self) {
        for dir in [
            self.home(),
            self.bin_dir(),
            self.fakes_dir(),
            self.tmp_dir(),
            self.other_machine_home(),
        ] {
            fs::create_dir_all(dir).unwrap();
        }
    }

    /// Who commits, and the urls of the fixture remotes. `~/.gitconfig` is the only git config
    /// read: `HOME` is the sandbox and the system one is turned off.
    fn write_git_config(&self) {
        let config = format!(
            "[user]\n\tname = You\n\temail = you@example.com\n\
             [init]\n\tdefaultBranch = main\n\
             [url \"{}/\"]\n\tinsteadOf = {GITHUB}\n",
            self.remotes_dir().display()
        );
        fs::write(self.home().join(".gitconfig"), config).unwrap();
    }

    /// `octok`, and fakes of `ssh`, `ssh-add` and `gh` that never reach the network.
    fn install_programs(&self) {
        symlink(env!("CARGO_BIN_EXE_octok"), self.bin_dir().join("octok")).unwrap();
        let fakes = [
            (
                "ssh",
                include_str!("../../../octotask-core/tests/fixtures/ssh"),
            ),
            (
                "ssh-add",
                include_str!("../../../octotask-core/tests/fixtures/ssh-add"),
            ),
            (
                "gh",
                include_str!("../../../octotask-core/tests/fixtures/gh"),
            ),
        ];
        for (name, script) in fakes {
            let path = self.bin_dir().join(name);
            fs::write(&path, script).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    /// Every host that `ssh` is given is one other machine, a folder of the sandbox whose user
    /// is `ubuntu`, with the key `~/.ssh/id_ed25519`.
    fn describe_other_machine(&self) {
        let key = self.home().join(".ssh/id_ed25519");
        fs::create_dir_all(key.parent().unwrap()).unwrap();
        fs::write(&key, "not a real key\n").unwrap();
        let config = format!(
            "hostname 10.0.0.5\nuser ubuntu\nport 22\nidentityfile {}\n",
            key.display()
        );
        fs::write(self.fakes_dir().join("ssh.config"), config).unwrap();
    }

    fn environment(&self) -> Vec<(&'static str, String)> {
        let path = format!(
            "{}:{}",
            self.bin_dir().display(),
            env::var("PATH").unwrap_or_default()
        );
        let example_commit_date = format!("{} +0000", NOW - EXAMPLE_COMMIT_AGE);
        let display = |path: PathBuf| path.display().to_string();
        vec![
            ("PATH", path),
            ("HOME", display(self.home())),
            ("OCTOTASK_HOME", display(self.home().join(".octotask"))),
            ("TMPDIR", display(self.tmp_dir())),
            ("GIT_CONFIG_NOSYSTEM", "1".to_string()),
            ("GIT_ALLOW_PROTOCOL", "file".to_string()),
            ("GIT_TEST_DATE_NOW", NOW.to_string()),
            ("GIT_AUTHOR_DATE", example_commit_date.clone()),
            ("GIT_COMMITTER_DATE", example_commit_date),
            ("FAKE_SSH_HOME", display(self.other_machine_home())),
            ("FAKE_SSH_LOG", display(self.fakes_dir().join("ssh.log"))),
            (
                "FAKE_SSH_CONFIG",
                display(self.fakes_dir().join("ssh.config")),
            ),
            (
                "FAKE_SSH_ADD_LOG",
                display(self.fakes_dir().join("ssh-add.log")),
            ),
            ("FAKE_GH_DIR", display(self.pull_requests_dir())),
        ]
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn remotes_dir(&self) -> PathBuf {
        self.root.join("remotes")
    }

    fn other_machine_home(&self) -> PathBuf {
        self.root.join("machine/home/ubuntu")
    }

    fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    fn fakes_dir(&self) -> PathBuf {
        self.root.join("fakes")
    }

    fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }
}

/// The remotes show as their GitHub url, the other machine as `/home/ubuntu`, the home as the
/// one of a reader, and anything else of the sandbox as `[sandbox]`. `root` may also be
/// written as `alias`, its path before symlinks were resolved.
fn redactions(root: &Path, alias: &Path) -> Vec<(String, String)> {
    let mut redactions = Vec::new();
    for root in [root, alias] {
        let root = root.display();
        redactions.extend([
            (format!("{root}/remotes/"), GITHUB.to_string()),
            (format!("{root}/machine"), String::new()),
            (format!("{root}/home"), READER_HOME.to_string()),
            (root.to_string(), "[sandbox]".to_string()),
        ]);
    }
    redactions
}
