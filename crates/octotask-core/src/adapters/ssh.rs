use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::utils::{Place, run_attached, run_checked};
use crate::config::{is_local, split_destination, validate_host};
use crate::{Error, Location, Result};

pub(super) const PROGRAM: &str = "ssh";
const SSH_ADD: &str = "ssh-add";

/// The exit status ssh keeps for its own failures, such as a host it cannot reach.
pub(super) const OWN_FAILURE: i32 = 255;

/// One octok command runs many short remote commands: they share one connection,
/// kept open for a minute, instead of each paying for a new login.
const SHARED_CONNECTION: [&str; 6] = [
    "-o",
    "ControlMaster=auto",
    "-o",
    "ControlPath=~/.ssh/octok-%C",
    "-o",
    "ControlPersist=60",
];

/// Makes ssh read the passphrase of a key from the macOS keychain, where
/// `Ssh::remember_passphrase` stores it. `IgnoreUnknown` is for an ssh that is not Apple's.
const KEYCHAIN: [&str; 4] = ["-o", "IgnoreUnknown=UseKeychain", "-o", "UseKeychain=yes"];

/// Folder of the octok home under the home of the remote user.
const DEFAULT_ROOT: &str = ".octotask";

/// One machine reached through the system `ssh`, which brings the user's `~/.ssh/config`.
#[derive(Debug, Clone)]
pub struct Ssh {
    host: String,
}

/// What ssh resolves for a host from `~/.ssh/config`, as `ssh -G` prints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshSettings {
    pub hostname: String,
    pub user: String,
    pub port: String,
    /// The private keys ssh would try, that exist as files.
    pub keys: Vec<PathBuf>,
}

impl Ssh {
    /// `host` is what `ssh` takes as its destination: a `Host` of `~/.ssh/config` or `user@host`.
    pub fn new(host: &str) -> Result<Self> {
        validate_host(host)?;
        Ok(Self {
            host: host.to_string(),
        })
    }

    /// Gives the key files ssh uses for this host to `ssh-add`, which asks for their passphrase
    /// at the terminal and keeps it: octok never sees it. Returns the keys, none when ssh
    /// uses no key file for this host.
    pub fn remember_passphrase(&self) -> Result<Vec<PathBuf>> {
        let keys = self.settings()?.keys;
        if keys.is_empty() {
            return Ok(keys);
        }
        let mut args: Vec<&str> = Vec::new();
        if cfg!(target_os = "macos") {
            args.push("--apple-use-keychain");
        }
        let paths: Vec<String> = keys
            .iter()
            .map(|key| key.to_string_lossy().into_owned())
            .collect();
        args.extend(paths.iter().map(String::as_str));
        run_attached(SSH_ADD, Place::here(), &args)?;
        Ok(keys)
    }

    /// Connects, finds the octok home and creates it. Returns its absolute path.
    /// `path` is absolute or relative to the home of the remote user; missing, it is `.octotask`.
    fn prepare_root(&self, path: Option<&str>) -> Result<PathBuf> {
        let home = self.home()?;
        let path = path.unwrap_or(DEFAULT_ROOT);
        let root = home.join(path.strip_prefix("~/").unwrap_or(path));
        let place = Place::host(&self.host);
        run_checked("mkdir", place, &["-p", "--", &root.to_string_lossy()])?;
        run_checked("git", place.in_dir(&root), &["--version"])?;
        Ok(root)
    }

    /// Home of the remote user: where a command run through ssh starts.
    fn home(&self) -> Result<PathBuf> {
        let printed = run_checked("pwd", Place::host(&self.host), &[])?;
        let home = printed.trim_end_matches('\n');
        if home.starts_with('/') && !home.contains('\n') {
            Ok(PathBuf::from(home))
        } else {
            Err(Error::UnexpectedOutput {
                command: format!("{PROGRAM} {} pwd", self.host),
                output: printed.trim().to_string(),
            })
        }
    }

    pub fn settings(&self) -> Result<SshSettings> {
        let config = run_checked(PROGRAM, Place::here(), &["-G", &self.host])?;
        let values = |name: &'static str| {
            config
                .lines()
                .filter_map(move |line| line.strip_prefix(name)?.strip_prefix(' '))
        };
        let first = |name| values(name).next().unwrap_or_default().to_string();
        Ok(SshSettings {
            hostname: first("hostname"),
            user: first("user"),
            port: first("port"),
            keys: values("identityfile")
                .map(expand_home)
                .filter(|key| key.is_file())
                .collect(),
        })
    }
}

/// Turns what the user writes into a location: `local`, `<host>` or `<host>:<path>`.
/// For another machine it connects once, to find the home of the user and create the folder.
pub fn resolve_location(input: &str) -> Result<Location> {
    let input = input.trim();
    if is_local(input) {
        return Ok(Location::Local);
    }
    let (host, path) = split_destination(input)?;
    let root = Ssh::new(host)?.prepare_root(path)?;
    Location::remote(host, root)
}

/// `ssh <host> <program> <args>`, with a `dir` passed to the program as `-C <dir>`.
/// A remote shell has no portable way to change directory first, and git, the only
/// program run in a directory, takes `-C`.
pub(super) fn command(host: &str, program: &str, dir: Option<&Path>, args: &[&str]) -> Command {
    let mut command = Command::new(PROGRAM);
    command.args(SHARED_CONNECTION);
    if cfg!(target_os = "macos") {
        command.args(KEYCHAIN);
    }
    if !io::stdin().is_terminal() {
        // Nobody is there to type a passphrase or accept a host key: fail instead of waiting.
        command.args(["-o", "BatchMode=yes"]);
    }
    command
        .arg("--")
        .arg(host)
        .arg(remote_command_line(program, dir, args))
        .stdin(Stdio::null());
    command
}

/// sshd hands the command to the remote user's shell as one string, so every word is quoted.
fn remote_command_line(program: &str, dir: Option<&Path>, args: &[&str]) -> String {
    let dir = dir.map(|dir| dir.to_string_lossy());
    // Fail instead of waiting for a password nobody may be there to type.
    let mut words = vec!["env", "GIT_TERMINAL_PROMPT=0", program];
    if let Some(dir) = &dir {
        words.extend(["-C", dir]);
    }
    words.extend(args);
    words
        .into_iter()
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(word: &str) -> String {
    let is_plain = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if !word.is_empty() && word.chars().all(is_plain) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// `ssh -G` prints the keys under the home as `~/...`.
fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::home_dir()) {
        (Some(relative), Some(home)) => home.join(relative),
        _ => PathBuf::from(path),
    }
}
