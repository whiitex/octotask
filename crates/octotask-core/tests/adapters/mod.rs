//! Tests for `src/adapters/`, one file per source file, plus the fixtures they share.

mod gh;
mod git;
mod ssh;
mod ssh_config;
mod utils;

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

const CHILD_MARKER: &str = "OCTOTASK_TEST_CHILD";

fn init_repo(dir: &Path) {
    git_setup(dir, &["init", "--quiet"]);
}

fn commit_empty(dir: &Path) {
    git_setup(
        dir,
        &[
            "-c",
            "user.name=octok",
            "-c",
            "user.email=octok@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "--no-verify",
            "--quiet",
            "--message",
            "init",
        ],
    );
}

fn canonical(path: &Path) -> String {
    path.canonicalize().unwrap().display().to_string()
}

/// Prepares fixtures with plain `git`, independently of the adapters under test.
fn git_setup(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

fn is_child() -> bool {
    env::var_os(CHILD_MARKER).is_some()
}

/// Runs one test again, in a child copy of the test binary with `vars` added,
/// so the environment change cannot leak into tests running in parallel.
/// The child takes the test's `is_child()` branch. `module` is the caller's `module_path!()`.
fn run_in_child(module: &str, test_name: &str, vars: &[(&str, &OsStr)]) {
    let module = module
        .split_once("::")
        .map_or("", |(_crate, module)| module);
    let output = Command::new(env::current_exe().unwrap())
        .args([format!("{module}::{test_name}").as_str(), "--exact"])
        .env(CHILD_MARKER, "1")
        .envs(vars.iter().copied())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "child run of {test_name} failed:\n{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A machine reached through the fake `ssh` of `tests/fixtures`: a folder of this one.
/// Its home has a space and a quote in its name, so every remote path needs quoting.
struct FakeRemote {
    tmp: TempDir,
}

impl FakeRemote {
    const HOME: &str = "FAKE_SSH_HOME";
    const LOG: &str = "FAKE_SSH_LOG";
    const CONFIG: &str = "FAKE_SSH_CONFIG";
    const ADD_LOG: &str = "FAKE_SSH_ADD_LOG";

    fn new() -> Self {
        let remote = Self {
            tmp: TempDir::new().unwrap(),
        };
        let bin = remote.bin_dir();
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(remote.home()).unwrap();
        fs::create_dir_all(remote.local_home()).unwrap();
        install(&bin.join("ssh"), include_str!("../fixtures/ssh"));
        install(&bin.join("ssh-add"), include_str!("../fixtures/ssh-add"));
        remote
    }

    fn bin_dir(&self) -> PathBuf {
        self.root().join("bin")
    }

    fn home(&self) -> PathBuf {
        self.root().join("remote user's home")
    }

    /// Home of the user on this side, where `~` in the path of a key points.
    fn local_home(&self) -> PathBuf {
        self.root().join("local home")
    }

    fn root(&self) -> PathBuf {
        self.tmp.path().canonicalize().unwrap()
    }

    /// The environment that puts the fake `ssh` first on `PATH`.
    fn vars(&self) -> Vec<(&'static str, OsString)> {
        let mut path = OsString::from(self.bin_dir());
        path.push(":");
        path.push(env::var_os("PATH").unwrap_or_default());
        vec![
            ("PATH", path),
            ("HOME", self.local_home().into()),
            (Self::HOME, self.home().into()),
            (Self::LOG, self.root().join("ssh.log").into()),
            (Self::CONFIG, self.root().join("ssh.config").into()),
            (Self::ADD_LOG, self.root().join("ssh-add.log").into()),
        ]
    }

    /// Runs the calling test again in a child that reaches this machine as any ssh host.
    fn run_in_child(&self, module: &str, test_name: &str) {
        let vars = self.vars();
        let vars: Vec<(&str, &OsStr)> = vars
            .iter()
            .map(|(name, value)| (*name, value.as_os_str()))
            .collect();
        run_in_child(module, test_name, &vars);
    }
}

/// In the child: a path the parent passed through the environment.
fn fake_remote_path(var: &str) -> PathBuf {
    PathBuf::from(env::var_os(var).unwrap())
}

fn install(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
