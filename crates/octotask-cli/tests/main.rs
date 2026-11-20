use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use rexpect::process::WaitStatus;
use rexpect::session::{PtySession, spawn_command};
use tempfile::TempDir;

struct Env {
    dir: TempDir,
    remotes: TempDir,
    /// Another machine, reached as any host through the fake `ssh` of the core tests:
    /// a folder of this one.
    machine: TempDir,
}

impl Env {
    fn new() -> Self {
        let env = Self {
            dir: TempDir::new().unwrap(),
            remotes: TempDir::new().unwrap(),
            machine: TempDir::new().unwrap(),
        };
        let bin = env.machine_dir().join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(env.machine_home()).unwrap();
        install(&bin, "ssh");
        install(&bin, "ssh-add");
        install(&bin, "gh");
        env
    }

    /// An open pull request of `repo`, for the fake `gh` of the core tests.
    fn pull_request(&self, repo: &str, number: u64, branch: &str, from_fork: bool) {
        let pull_request = serde_json::json!({
            "number": number,
            "title": format!("Change {number}"),
            "headRefName": branch,
            "author": {"login": "alice"},
            "isCrossRepository": from_fork,
        });
        self.gh_file(repo, &format!("{number}.json"), &pull_request.to_string());
    }

    /// A file of the fake `gh` for `repo`: a pull request, or the `error` it fails with.
    fn gh_file(&self, repo: &str, name: &str, content: &str) {
        let dir = self.machine_file("pull requests").join(repo);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(name), content).unwrap();
    }

    /// A project whose backend has the pull request 12 for `feat/login` and whose agent
    /// has the branch `fix/prompt`. The frontend has neither.
    fn with_a_pull_request() -> Self {
        let env = Self::with_repos(&["frontend", "backend", "agent"]);
        commit_on_new_branch(&env.remote_dir("backend"), "feat/login", COMMIT_DATE);
        commit_on_new_branch(&env.remote_dir("agent"), "fix/prompt", COMMIT_DATE);
        env.pull_request("backend", 12, "feat/login", false);
        env
    }

    fn machine_dir(&self) -> PathBuf {
        self.machine.path().canonicalize().unwrap()
    }

    /// Home of the user on the other machine. The space and the quote make every path
    /// there need quoting.
    fn machine_home(&self) -> PathBuf {
        self.machine_dir().join("remote user's home")
    }

    /// The octok home on the other machine.
    fn machine_root(&self) -> PathBuf {
        self.machine_home().join(".octotask")
    }

    /// The location octok saves for the other machine reached as `host`.
    fn machine_location(&self, host: &str) -> String {
        format!("{host}:{}", self.machine_root().display())
    }

    fn machine_file(&self, name: &str) -> PathBuf {
        self.machine_dir().join(name)
    }

    fn remote_dir(&self, name: &str) -> PathBuf {
        self.remotes.path().join(format!("{name}.git"))
    }

    fn remote_url(&self, name: &str) -> String {
        self.remote_dir(name).display().to_string()
    }

    /// Creates a local repo with one commit on `main`, to be used as a remote without network.
    fn remote(&self, name: &str) -> String {
        let dir = self.remote_dir(name);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "--quiet", "--initial-branch", "main"]);
        commit(&dir, "init");
        self.remote_url(name)
    }

    /// Like `remote`, with one more branch at the same commit.
    fn remote_with_branch(&self, name: &str, branch: &str) -> String {
        let url = self.remote(name);
        git(&self.remote_dir(name), &["branch", branch]);
        url
    }

    /// A project named `deeptree` with one cloned repo per name.
    fn with_repos(names: &[&str]) -> Self {
        let env = Self::new();
        env.octok(&["project", "init", "deeptree", "--base", "main"])
            .assert()
            .success();
        for name in names {
            env.octok(&["repo", "add", &env.remote(name)])
                .assert()
                .success();
        }
        env
    }

    fn workspace_dir(&self, name: &str) -> PathBuf {
        self.dir.path().join("workspaces/deeptree").join(name)
    }

    fn stdout(&self, args: &[&str]) -> String {
        let out = self.octok(args).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }

    fn octok(&self, args: &[&str]) -> Command {
        let mut path = OsString::from(self.machine_dir().join("bin"));
        path.push(":");
        path.push(env::var_os("PATH").unwrap_or_default());
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("octok"));
        cmd.env("OCTOTASK_HOME", self.dir.path())
            .env("PATH", path)
            .env("FAKE_SSH_HOME", self.machine_home())
            .env("FAKE_SSH_LOG", self.machine_file("ssh.log"))
            .env("FAKE_SSH_CONFIG", self.machine_file("ssh.config"))
            .env("FAKE_SSH_ADD_LOG", self.machine_file("ssh-add.log"))
            .env("FAKE_GH_DIR", self.machine_file("pull requests"))
            .args(args);
        cmd
    }

    fn json(&self, args: &[&str]) -> serde_json::Value {
        let out = self.octok(args).output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap()
    }

    /// The exit code of `octok`, and the one JSON document it must print on stdout.
    fn document(&self, args: &[&str]) -> (Option<i32>, serde_json::Value) {
        let out = self.octok(args).output().unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 1, "one document: {stdout:?}");
        let document =
            serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout:?}"));
        (out.status.code(), document)
    }

    fn read(&self, relative_path: &str) -> String {
        fs::read_to_string(self.dir.path().join(relative_path)).unwrap()
    }

    /// Runs `octok` at a terminal, where it asks for what the flags leave out.
    fn terminal(&self, args: &[&str]) -> PtySession {
        let mut octok = self.octok(args);
        octok.env("HOME", self.terminal_home());
        spawn_command(octok, Some(TERMINAL_TIMEOUT_MS)).unwrap()
    }

    /// Home of the user at the terminal, so the tests never read the real `~/.ssh/config`.
    fn terminal_home(&self) -> PathBuf {
        self.machine_file("local home")
    }

    fn write_ssh_config(&self, text: &str) {
        let ssh_dir = self.terminal_home().join(".ssh");
        fs::create_dir_all(&ssh_dir).unwrap();
        fs::write(ssh_dir.join("config"), text).unwrap();
    }

    /// The other machine as `ssh -G` describes it, with one key file.
    fn describe_machine_to_ssh(&self) -> PathBuf {
        let key = self.machine_file("id_test");
        fs::write(&key, "a private key").unwrap();
        fs::write(
            self.machine_file("ssh.config"),
            format!(
                "hostname 10.0.0.5\nuser ubuntu\nport 2222\nidentityfile {}\n",
                key.display()
            ),
        )
        .unwrap();
        key
    }

    fn project_dir_entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.dir.path().join("projects"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

const COMMIT_DATE: &str = "2020-01-01T00:00:00Z";

const TERMINAL_TIMEOUT_MS: u64 = 20_000;

/// Types `line` once `octok` has asked `question`.
fn answer(terminal: &mut PtySession, question: &str, line: &str) {
    terminal.exp_string(question).unwrap();
    terminal.send_line(line).unwrap();
}

/// The JSON document that ends what a terminal shows: git writes its progress before it.
fn last_document(printed: &str) -> serde_json::Value {
    let line = printed.lines().last().unwrap_or_default();
    serde_json::from_str(line).unwrap_or_else(|error| panic!("{error}: {printed:?}"))
}

/// Waits for `octok` to end well and returns what it still printed.
fn finish(mut terminal: PtySession) -> String {
    let printed = terminal.exp_eof().unwrap();
    let status = terminal.process().wait().unwrap();
    assert!(
        matches!(status, WaitStatus::Exited(_, 0)),
        "{status:?}: {printed}"
    );
    printed
}

/// Runs plain `git`, independently of the binary under test, and returns its trimmed stdout.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        // A fixed, old date: "2 seconds ago" would turn into "3 seconds ago" mid-test.
        .env("GIT_AUTHOR_DATE", COMMIT_DATE)
        .env("GIT_COMMITTER_DATE", COMMIT_DATE)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Links a fake program of the core fixtures into `bin`.
/// A link and not a copy: Linux refuses to run a file that a process has open for writing
/// ("Text file busy"), and a test that forks in parallel holds a copy open for a moment.
fn install(bin: &Path, program: &str) {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../octotask-core/tests/fixtures")
        .join(program);
    symlink(fixture, bin.join(program)).unwrap();
}

/// An empty commit on a new `branch`, made at `date`.
fn commit_on_new_branch(dir: &Path, branch: &str, date: &str) {
    git(dir, &["checkout", "--quiet", "-b", branch]);
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=octok",
            "-c",
            "user.email=octok@example.com",
        ])
        .args(["-c", "commit.gpgsign=false", "commit", "--allow-empty"])
        .args(["--no-verify", "--quiet", "--message", branch])
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .status()
        .unwrap();
    assert!(status.success());
}

fn commit(dir: &Path, message: &str) {
    git(
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
            message,
        ],
    );
}

#[test]
fn registered_repos_are_persisted_in_project_file() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "develop"])
        .assert()
        .success();
    env.octok(&[
        "repo",
        "add",
        &env.remote_with_branch("frontend", "develop"),
    ])
    .assert()
    .success();
    env.octok(&[
        "repo",
        "add",
        &env.remote("financial-agent"),
        "--name",
        "agent",
        "--base",
        "main",
    ])
    .assert()
    .success();

    let raw = fs::read_to_string(env.dir.path().join("projects/deeptree.yaml")).unwrap();
    assert!(raw.contains(&env.remote_url("frontend")), "{raw}");

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["schema"], 1);
    assert_eq!(ls["project"], "deeptree");
    assert_eq!(
        ls["repos"],
        serde_json::json!([
            {"key": "frontend", "alias": null, "url": env.remote_url("frontend"), "base": "develop", "location": "local"},
            {"key": "agent", "alias": null, "url": env.remote_url("financial-agent"), "base": "main", "location": "local"},
        ])
    );
}

#[test]
fn repo_commands_target_default_project_unless_overridden() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "init", "side", "--base", "main"])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote("tool")])
        .assert()
        .success();
    env.octok(&["-p", "deeptree", "repo", "add", &env.remote("frontend")])
        .assert()
        .success();

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"][0]["key"],
        "tool"
    );
    assert_eq!(
        env.json(&["repo", "ls", "-p", "deeptree", "--json"])["repos"][0]["key"],
        "frontend"
    );

    env.octok(&["project", "use", "deeptree"])
        .assert()
        .success();
    assert_eq!(env.json(&["repo", "ls", "--json"])["project"], "deeptree");
}

#[test]
fn project_init_announces_the_new_default() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "init", "side", "--base", "main"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "`side` is now the default project\n",
        ));
}

#[test]
fn project_rm_of_another_project_keeps_the_default() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "init", "side", "--base", "main"])
        .assert()
        .success();

    env.octok(&["project", "rm", "deeptree"])
        .assert()
        .success()
        .stdout("removed project `deeptree`\n");

    let ls = env.json(&["project", "ls", "--json"]);
    assert_eq!(ls["projects"], serde_json::json!(["side"]));
    assert_eq!(ls["default"], "side");

    assert_eq!(env.project_dir_entries(), ["side.yaml"]);
    assert!(env.read("config.yaml").contains("default_project: side"));
}

#[test]
fn project_rm_of_the_default_leaves_no_default() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "init", "side", "--base", "main"])
        .assert()
        .success();

    env.octok(&["project", "rm", "side"])
        .assert()
        .success()
        .stdout(
            "removed project `side`\n\
             no default project: choose one with `octok project use <name>`\n",
        );

    let ls = env.json(&["project", "ls", "--json"]);
    assert_eq!(ls["projects"], serde_json::json!(["deeptree"]));
    assert_eq!(ls["default"], serde_json::Value::Null);
    assert_eq!(env.project_dir_entries(), ["deeptree.yaml"]);
    assert!(!env.read("config.yaml").contains("default_project"));

    env.octok(&["repo", "ls"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("default project"));
    env.octok(&["project", "rm", "side"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("project `side` not found"));
}

#[test]
fn project_ls_reports_default() {
    let env = Env::new();
    env.octok(&["project", "init", "zeta", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "init", "alpha", "--base", "main"])
        .assert()
        .success();
    let ls = env.json(&["project", "ls", "--json"]);
    assert_eq!(ls["schema"], 1);
    assert_eq!(ls["projects"], serde_json::json!(["alpha", "zeta"]));
    assert_eq!(ls["default"], "alpha");
}

#[test]
fn project_ls_text_marks_default() {
    let env = Env::new();
    env.octok(&["project", "ls"])
        .assert()
        .success()
        .stdout("no projects yet: create one with `octok project init <name>`\n");

    env.octok(&["project", "init", "zeta", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "init", "alpha", "--base", "main"])
        .assert()
        .success();
    env.octok(&["project", "ls"])
        .assert()
        .success()
        .stdout("* alpha\n  zeta\n");
}

#[test]
fn repo_ls_text_aligns_columns_and_shows_effective_base() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "develop"])
        .assert()
        .success();
    env.octok(&["repo", "ls"])
        .assert()
        .success()
        .stdout("no repos in deeptree: add one with `octok repo add <url>`\n");

    env.octok(&[
        "repo",
        "add",
        &env.remote_with_branch("frontend", "develop"),
    ])
    .assert()
    .success();
    env.octok(&["repo", "add", &env.remote("api"), "--base", "main"])
        .assert()
        .success();
    env.octok(&["repo", "ls"])
        .assert()
        .success()
        .stdout(format!(
            "frontend  develop  {}\napi       main     {}\n",
            env.remote_url("frontend"),
            env.remote_url("api")
        ));
}

#[test]
fn repo_rm_removes_from_file() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote("frontend")])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote("backend")])
        .assert()
        .success();

    env.octok(&["repo", "rm", "frontend"]).assert().success();

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([
            {"key": "backend", "alias": null, "url": env.remote_url("backend"), "base": "main", "location": "local"},
        ])
    );
    let after_rm = env.read("projects/deeptree.yaml");
    assert!(!after_rm.contains("frontend"), "{after_rm}");
    assert!(after_rm.contains(&env.remote_url("backend")), "{after_rm}");

    env.octok(&["repo", "rm", "frontend"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("frontend"));
    assert_eq!(env.read("projects/deeptree.yaml"), after_rm);
}

#[test]
fn concurrent_repo_adds_are_all_saved() {
    const REPOS: usize = 24;
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();

    let children: Vec<_> = (0..REPOS)
        .map(|i| {
            env.octok(&["repo", "add", &env.remote(&format!("repo{i}"))])
                // `spawn` passes on the terminal of whoever runs the tests, and at a
                // terminal `octok` asks questions.
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["repos"].as_array().unwrap().len(), REPOS);
}

#[test]
fn repo_add_with_name_accepts_url_whose_last_segment_is_not_a_valid_key() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote(".github"), "--name", "dotgithub"])
        .assert()
        .success();

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([
            {"key": "dotgithub", "alias": null, "url": env.remote_url(".github"), "base": "main", "location": "local"},
        ])
    );
}

#[test]
fn repo_add_without_name_asks_for_one_when_no_key_can_be_derived() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    let before = env.read("projects/deeptree.yaml");

    env.octok(&["repo", "add", &env.remote(".github")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--name"))
        .stderr(predicate::str::contains("invalid git url").not());

    assert_eq!(env.read("projects/deeptree.yaml"), before);
}

#[test]
fn newer_global_config_is_rejected_and_left_untouched() {
    let env = Env::new();
    let config = env.dir.path().join("config.yaml");
    fs::write(&config, "version: 99\n").unwrap();

    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unsupported config version 99 (expected 1)",
        ));

    assert_eq!(fs::read_to_string(&config).unwrap(), "version: 99\n");
    assert!(!env.dir.path().join("projects/deeptree.yaml").exists());
}

#[test]
fn errors_exit_with_a_message_and_change_nothing() {
    let env = Env::new();
    env.octok(&["repo", "add", &env.remote("frontend")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("default project"));

    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["repo", "add", "not-a-url"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not-a-url"));
    env.octok(&["repo", "add", &env.remote("frontend")])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote("other/frontend")])
        .assert()
        .code(5);

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["repos"].as_array().unwrap().len(), 1);
    assert_eq!(ls["repos"][0]["url"], env.remote_url("frontend"));
}

#[test]
fn repo_add_clones_into_the_cache_and_repo_rm_deletes_it() {
    let env = Env::with_repos(&["frontend"]);
    let cache = env.dir.path().join("cache/deeptree/frontend.git");
    assert_eq!(git(&cache, &["rev-parse", "--is-bare-repository"]), "true");

    env.octok(&["repo", "rm", "frontend"]).assert().success();

    assert!(!cache.exists());
}

#[test]
fn repo_add_with_unreachable_url_registers_nothing() {
    let env = Env::with_repos(&[]);

    env.octok(&["repo", "add", &env.remote_url("missing")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("git fetch"));

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([])
    );
}

#[test]
fn repo_rm_is_refused_while_a_workspace_uses_the_repo() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["repo", "rm", "frontend"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("feat-login"));
}

#[test]
fn fetch_reports_every_repo_and_exits_6_when_one_fails() {
    let env = Env::with_repos(&["frontend", "backend"]);
    commit(&env.remote_dir("backend"), "second");

    env.octok(&["fetch"])
        .assert()
        .success()
        .stdout("frontend  fetched\nbackend   fetched\n");
    let cache = env.dir.path().join("cache/deeptree/backend.git");
    assert_eq!(
        git(&cache, &["rev-parse", "refs/remotes/origin/main"]),
        git(&env.remote_dir("backend"), &["rev-parse", "main"])
    );

    fs::remove_dir_all(env.remote_dir("frontend")).unwrap();
    env.octok(&["fetch"])
        .assert()
        .code(6)
        .stdout("frontend  failed\nbackend   fetched\n")
        .stderr(predicate::str::starts_with("error: frontend: "));
}

#[test]
fn branch_creates_a_workspace_and_prints_only_its_path_on_stdout() {
    let env = Env::with_repos(&["frontend", "backend"]);
    let path = env.workspace_dir("feat-login");

    env.octok(&["branch", "feat/login"])
        .assert()
        .success()
        .stdout(format!("{}\n", path.display()))
        .stderr("created workspace `feat-login` in deeptree\n");

    for repo in ["frontend", "backend"] {
        assert_eq!(
            git(&path.join(repo), &["branch", "--show-current"]),
            "feat/login"
        );
    }
}

#[test]
fn checkout_b_creates_and_plain_checkout_prints_the_path() {
    let env = Env::with_repos(&["frontend"]);
    let path = format!("{}\n", env.workspace_dir("feat-login").display());

    env.octok(&["checkout", "feat/login"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("octok checkout -b feat/login"));

    assert_eq!(env.stdout(&["checkout", "-b", "feat/login"]), path);
    assert_eq!(env.stdout(&["checkout", "feat/login"]), path);
}

#[test]
fn checkout_creates_the_workspace_of_a_branch_that_is_already_on_a_remote() {
    let env = Env::with_repos(&["frontend", "backend"]);
    commit_on_new_branch(
        &env.remote_dir("backend"),
        "feat/login",
        "2023-01-01T00:00:00Z",
    );
    let path = env.workspace_dir("feat-login");

    env.octok(&["checkout", "feat/login"])
        .assert()
        .success()
        .stdout(format!("{}\n", path.display()))
        .stderr(
            "created workspace `feat-login` in deeptree\n  \
             branch from the remote: backend\n  \
             new branch from the base: frontend\n",
        );

    let head = |dir: &Path, rev: &str| git(dir, &["rev-parse", rev]);
    assert_eq!(
        head(&path.join("backend"), "HEAD"),
        head(&env.remote_dir("backend"), "feat/login")
    );
    assert_eq!(
        git(&path.join("frontend"), &["branch", "--show-current"]),
        "feat/login"
    );
    assert_eq!(
        head(&path.join("frontend"), "HEAD"),
        head(&env.remote_dir("frontend"), "main")
    );
    env.octok(&["checkout", "feat/login"])
        .assert()
        .success()
        .stdout(format!("{}\n", path.display()))
        .stderr("");
    env.octok(&["rm", "feat/login"]).assert().success();
}

#[test]
fn checkout_of_a_branch_that_is_on_no_remote_is_refused_with_a_hint() {
    let env = Env::with_repos(&["frontend", "backend"]);

    env.octok(&["checkout", "feat/nope"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(
            "error: no repo has a branch `feat/nope` on its remote: \
             create it with `octok checkout -b feat/nope`\n",
        );

    assert!(!env.workspace_dir("feat-nope").exists());
}

#[test]
fn checkout_puts_the_workspace_of_an_existing_branch_on_another_machine() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    let url = env.remote_with_branch("frontend", "feat/there");
    env.octok(&["repo", "add", &url]).assert().success();
    let there = env.machine_root().join("workspaces/deeptree/feat-there");

    env.octok(&["checkout", "feat/there", "--location", "vm"])
        .assert()
        .success()
        .stdout(format!("vm:{}\n", there.display()));

    assert_eq!(
        git(&there.join("frontend"), &["branch", "--show-current"]),
        "feat/there"
    );
}

#[test]
fn status_marks_the_checkouts_whose_branch_is_not_on_the_remote() {
    let env = Env::with_repos(&["frontend", "backend"]);
    git(&env.remote_dir("backend"), &["branch", "feat/login"]);
    env.octok(&["checkout", "feat/login"]).assert().success();

    let rows: Vec<String> = env
        .stdout(&["status"])
        .lines()
        .filter(|line| line.contains("feat/login"))
        .map(str::to_string)
        .collect();
    let checkouts = env.json(&["status", "--json"])["workspaces"][0]["checkouts"].clone();

    assert!(rows[0].ends_with("  [not on the remote]"), "{}", rows[0]);
    assert!(!rows[1].contains("not on the remote"), "{}", rows[1]);
    assert_eq!(checkouts[0]["on_remote"], false);
    assert_eq!(checkouts[1]["on_remote"], true);
}

#[test]
fn branch_that_already_exists_is_refused_with_a_hint() {
    let env = Env::with_repos(&["frontend", "backend"]);
    git(&env.remote_dir("backend"), &["branch", "feat/login"]);

    env.octok(&["branch", "feat/login"])
        .assert()
        .code(5)
        .stdout("")
        .stderr(
            "error: branch `feat/login` already exists on the remote of: backend\n\
             check it out with `octok checkout feat/login`, or choose another name\n",
        );

    assert!(!env.workspace_dir("feat-login").exists());
}

#[test]
fn pull_uses_the_workspace_of_the_current_directory() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    let remote = env.remote_dir("frontend");
    git(&remote, &["checkout", "--quiet", "-b", "feat/login"]);
    commit(&remote, "pushed by a teammate");
    let worktree = env.workspace_dir("feat-login").join("frontend");

    // Another default project must not matter: the directory decides.
    env.octok(&["project", "init", "side", "--base", "main"])
        .assert()
        .success();
    env.octok(&["pull"])
        .current_dir(&worktree)
        .assert()
        .success()
        .stdout(
            "frontend  updated\n\
             backend   skipped: branch is not on the remote yet\n",
        );

    assert_eq!(
        git(&worktree, &["rev-parse", "HEAD"]),
        git(&remote, &["rev-parse", "feat/login"])
    );
}

#[test]
fn pull_takes_a_branch_and_needs_one_outside_a_workspace() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["pull", "feat/login"])
        .current_dir(env.dir.path())
        .assert()
        .success()
        .stdout("frontend  skipped: branch is not on the remote yet\n");
    env.octok(&["pull"])
        .current_dir(env.dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("octok pull <branch>"));
}

#[test]
fn status_json_lists_repos_and_workspace_checkouts() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    let workspace = env.workspace_dir("feat-login");
    commit(&workspace.join("frontend"), "add login page");

    let status = env.json(&["status", "--json"]);

    assert_eq!(status["schema"], 1);
    assert_eq!(status["project"], "deeptree");
    let repo = &status["repos"][0];
    assert_eq!(repo["key"], "frontend");
    assert_eq!(repo["url"], env.remote_url("frontend"));
    assert_eq!(repo["base"], "main");
    assert_eq!(repo["commit"]["subject"], "init");
    let listed = &status["workspaces"][0];
    assert_eq!(listed["name"], "feat-login");
    assert_eq!(listed["path"], workspace.display().to_string());
    let checkout = &listed["checkouts"][0];
    assert_eq!(checkout["repo"], "frontend");
    assert_eq!(checkout["branch"], "feat/login");
    assert_eq!(checkout["commit"]["subject"], "add login page");
    assert_eq!(
        checkout["commit"]["sha"],
        git(
            &workspace.join("frontend"),
            &["rev-parse", "--short", "HEAD"]
        )
    );
}

#[test]
fn status_text_shows_project_repos_and_workspaces() {
    let env = Env::with_repos(&[]);
    env.octok(&["status"]).assert().success().stdout(
        "project: deeptree\n\n\
         repos: none (add one with `octok repo add <url>`)\n\n\
         workspaces: none (create one with `octok branch <name>`)\n",
    );

    env.octok(&["repo", "add", &env.remote("frontend")])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();
    let sha = git(
        &env.remote_dir("frontend"),
        &["rev-parse", "--short", "main"],
    );
    let age = git(&env.remote_dir("frontend"), &["log", "-1", "--format=%cr"]);
    let line = format!("{sha}  init ({age})");

    env.octok(&["status"]).assert().success().stdout(format!(
        "project: deeptree\n\n\
         repos:\n  frontend  main  {line}\n\n\
         workspaces:\n  feat-login  {}\n    \
         frontend  feat/login  {line}  [not on the remote]\n",
        env.workspace_dir("feat-login").display()
    ));
}

#[test]
fn repo_alias_is_shown_and_accepted_by_repo_rm() {
    let env = Env::with_repos(&[]);
    env.octok(&[
        "repo",
        "add",
        &env.remote("DeepTreeDashboard"),
        "--alias",
        "frontend",
    ])
    .assert()
    .success()
    .stdout(predicate::str::starts_with(
        "added `DeepTreeDashboard (frontend)` to deeptree: ",
    ));
    env.octok(&["repo", "add", &env.remote("MCPServer")])
        .assert()
        .success();

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["repos"][0]["key"], "DeepTreeDashboard");
    assert_eq!(ls["repos"][0]["alias"], "frontend");
    env.octok(&["repo", "ls"])
        .assert()
        .success()
        .stdout(format!(
            "DeepTreeDashboard (frontend)  main  {}\nMCPServer                     main  {}\n",
            env.remote_url("DeepTreeDashboard"),
            env.remote_url("MCPServer")
        ));
    env.octok(&["fetch"]).assert().success().stdout(
        "DeepTreeDashboard (frontend)  fetched\n\
         MCPServer                     fetched\n",
    );
    assert_eq!(
        env.json(&["status", "--json"])["repos"][0]["alias"],
        "frontend"
    );

    env.octok(&["repo", "rm", "frontend"])
        .assert()
        .success()
        .stdout("removed `DeepTreeDashboard` from deeptree\n");
    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"][0]["key"],
        "MCPServer"
    );
}

#[test]
fn duplicate_repo_names_and_aliases_are_explained() {
    let env = Env::with_repos(&[]);
    let first = env.remote("org/DeepTreeDashboard");
    env.octok(&["repo", "add", &first, "--alias", "frontend"])
        .assert()
        .success();

    env.octok(&["repo", "add", &env.remote("fork/DeepTreeDashboard")])
        .assert()
        .code(5)
        .stderr(format!(
            "error: repo `DeepTreeDashboard` is already registered ({first})\n\
             choose another name with `--name <key>`\n"
        ));
    env.octok(&[
        "repo",
        "add",
        &env.remote_url("fork/DeepTreeDashboard"),
        "--name",
        "DeepTreeDashboard-fork",
        "--alias",
        "frontend",
    ])
    .assert()
    .code(5)
    .stderr("error: alias `frontend` is already used by repo `DeepTreeDashboard`\n");

    env.octok(&[
        "repo",
        "add",
        &env.remote_url("fork/DeepTreeDashboard"),
        "--name",
        "DeepTreeDashboard-fork",
    ])
    .assert()
    .success();
    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn status_text_aligns_hashes_and_lists_orphan_caches() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&["repo", "add", &env.remote("old")])
        .assert()
        .success();
    let cache = env.dir.path().join("cache/deeptree");
    // git abbreviates hashes to different lengths depending on the repo.
    git(&cache.join("backend.git"), &["config", "core.abbrev", "12"]);
    let project_file = env.dir.path().join("projects/deeptree.yaml");
    let without_old = env
        .read("projects/deeptree.yaml")
        .replace(&format!("  old:\n    url: {}\n", env.remote_url("old")), "");
    fs::write(&project_file, without_old).unwrap();
    let short = git(
        &env.remote_dir("frontend"),
        &["rev-parse", "--short=7", "main"],
    );
    let long = git(
        &env.remote_dir("backend"),
        &["rev-parse", "--short=12", "main"],
    );

    let out = env.stdout(&["status"]);

    assert!(
        out.contains(&format!("  frontend  main  {short}       init (")),
        "{out}"
    );
    assert!(
        out.contains(&format!("  backend   main  {long}  init (")),
        "{out}"
    );
    assert!(
        out.ends_with(&format!(
            "\norphan caches (not in the project file):\n  {}\n",
            cache.join("old.git").display()
        )),
        "{out}"
    );
}

#[test]
fn repo_add_without_a_terminal_refuses_a_base_missing_on_the_remote() {
    let env = Env::with_repos(&[]);

    env.octok(&["repo", "add", &env.remote("frontend"), "--base", "nope"])
        .assert()
        .code(3)
        .stderr("error: repo `frontend` has no branch `nope` on its remote\n");

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([])
    );
    assert!(!env.dir.path().join("cache/deeptree/frontend.git").exists());
}

#[test]
fn status_text_explains_a_base_branch_missing_on_the_remote() {
    let env = Env::with_repos(&["frontend"]);
    let project_file = env.dir.path().join("projects/deeptree.yaml");
    let edited = env
        .read("projects/deeptree.yaml")
        .replace("base: main", "base: nope");
    fs::write(&project_file, edited).unwrap();

    let out = env.stdout(&["status"]);

    assert!(
        out.contains("  frontend  nope  branch `nope` is not on the remote\n"),
        "{out}"
    );
    assert_eq!(env.json(&["status", "--json"])["repos"][0]["cached"], true);
}

#[test]
fn branch_on_an_existing_workspace_adds_the_repos_that_joined_later() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    env.octok(&["repo", "add", &env.remote("backend")])
        .assert()
        .success();
    let path = env.workspace_dir("feat-login");

    env.octok(&["branch", "feat/login"])
        .assert()
        .success()
        .stdout(format!("{}\n", path.display()))
        .stderr(
            "completed workspace `feat-login` in deeptree\n\
             \x20 new branch from the base: backend\n",
        );
    env.octok(&["branch", "feat/login"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("already exists"));

    fs::remove_dir_all(path.join("frontend")).unwrap();
    env.octok(&["branch", "feat/login"])
        .assert()
        .success()
        .stderr(
            "completed workspace `feat-login` in deeptree\n\
         \x20 branch restored from the cache: frontend\n",
        );
}

#[test]
fn rm_deletes_a_workspace_unless_it_holds_unsaved_work() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    let path = env.workspace_dir("feat-login");
    fs::write(path.join("frontend/draft.txt"), "wip").unwrap();
    commit(&path.join("backend"), "local work");

    env.octok(&["rm", "feat/login"]).assert().code(4).stderr(
        "error: workspace `feat-login` has work that would be lost:\n\
         \x20 frontend: uncommitted changes\n\
         \x20 backend: 1 unpushed commit on `feat/login`\n\
         remove it anyway with `octok rm --force <branch>`\n",
    );
    assert!(path.join("frontend/draft.txt").is_file());

    env.octok(&["rm", "--force", "feat/login"])
        .assert()
        .success()
        .stdout("removed workspace `feat-login` from deeptree\n");
    assert!(!path.exists());
    env.octok(&["rm", "feat/login"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("no workspace for `feat/login`"));
    env.octok(&["branch", "feat/login"]).assert().success();
}

#[test]
fn project_rm_is_refused_with_workspaces_and_then_deletes_the_caches() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["project", "rm", "deeptree"])
        .assert()
        .code(5)
        .stderr(
            "error: project `deeptree` still has workspaces: feat-login\n\
             remove them first with `octok rm <branch>`\n",
        );
    assert_eq!(env.project_dir_entries(), ["deeptree.yaml"]);

    env.octok(&["rm", "feat/login"]).assert().success();
    env.octok(&["project", "rm", "deeptree"]).assert().success();
    assert!(!env.dir.path().join("cache/deeptree").exists());

    // A new project with the same name starts empty.
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    env.octok(&["status"]).assert().success().stdout(
        "project: deeptree\n\n\
         repos: none (add one with `octok repo add <url>`)\n\n\
         workspaces: none (create one with `octok branch <name>`)\n",
    );
}

#[test]
fn status_text_shows_a_broken_worktree_next_to_the_healthy_ones() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    fs::remove_dir_all(env.dir.path().join("cache/deeptree/backend.git/worktrees")).unwrap();

    env.octok(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("    frontend  feat/login  "))
        .stdout(predicate::str::contains(
            "    backend   cannot be read: fatal: ",
        ));
}

#[test]
fn status_text_points_out_a_repo_without_worktree_in_a_workspace() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    env.octok(&["repo", "add", &env.remote("backend")])
        .assert()
        .success();

    env.octok(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "    backend   no worktree: add it with `octok checkout feat/login`\n",
        ));
}

#[test]
fn repo_set_changes_alias_base_and_url() {
    let env = Env::with_repos(&["frontend"]);
    git(&env.remote_dir("frontend"), &["branch", "develop"]);
    let fork = env.remote_with_branch("fork", "develop");

    env.octok(&[
        "repo", "set", "frontend", "--alias", "web", "--base", "develop",
    ])
    .assert()
    .success()
    .stdout("updated `frontend (web)` in deeptree\n");
    env.octok(&["repo", "set", "web", "--url", &fork])
        .assert()
        .success();
    assert_eq!(
        env.stdout(&["repo", "ls"]),
        format!("frontend (web)  develop  {fork}\n")
    );

    env.octok(&["repo", "set", "web", "--no-alias", "--no-base"])
        .assert()
        .success()
        .stdout("updated `frontend` in deeptree\n");
    assert_eq!(
        env.stdout(&["repo", "ls"]),
        format!("frontend  main  {fork}\n")
    );
}

#[test]
fn repo_set_rejects_missing_or_contradictory_options_and_unknown_bases() {
    let env = Env::with_repos(&["frontend"]);

    env.octok(&["repo", "set", "frontend"]).assert().code(2);
    env.octok(&["repo", "set", "frontend", "--alias", "web", "--no-alias"])
        .assert()
        .code(2);
    env.octok(&["repo", "set", "frontend", "--base", "nope"])
        .assert()
        .code(3)
        .stderr("error: repo `frontend` has no branch `nope` on its remote\n");
    env.octok(&["repo", "set", "missing", "--alias", "web"])
        .assert()
        .code(3)
        .stderr("error: repo `missing` is not registered\n");
    assert_eq!(
        env.stdout(&["repo", "ls"]),
        format!("frontend  main  {}\n", env.remote_url("frontend"))
    );
}

#[test]
fn project_set_changes_the_base_and_warns_about_repos_without_it() {
    let env = Env::with_repos(&["frontend", "backend"]);
    git(&env.remote_dir("backend"), &["branch", "develop"]);
    env.octok(&["fetch"]).assert().success();

    env.octok(&["project", "set", "--base", "develop"])
        .assert()
        .success()
        .stdout("base of `deeptree` is now `develop`\n")
        .stderr("warning: branch `develop` is not on the remote of: frontend\n");

    assert!(env.read("projects/deeptree.yaml").contains("base: develop"));
    env.octok(&["project", "set"]).assert().code(2);
}

#[test]
fn project_init_with_a_location_finds_the_octok_home_on_the_other_machine() {
    let env = Env::new();
    env.octok(&[
        "project",
        "init",
        "deeptree",
        "--base",
        "main",
        "--location",
        "vm",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains(env.machine_location("vm")));
    assert!(env.machine_root().is_dir());

    env.octok(&["repo", "add", &env.remote("backend")])
        .assert()
        .success();

    let repos = env.json(&["repo", "ls", "--json"])["repos"].clone();
    assert_eq!(repos[0]["location"], env.machine_location("vm"));
    assert!(
        env.machine_root()
            .join("cache/deeptree/backend.git")
            .is_dir()
    );
    assert!(!env.dir.path().join("cache/deeptree/backend.git").exists());
}

#[test]
fn a_repo_on_another_machine_is_cloned_there() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success()
        .stdout(predicate::str::ends_with(format!(
            "cloned at {}\n",
            env.machine_location("vm")
        )));

    assert!(
        env.machine_root()
            .join("cache/deeptree/backend.git")
            .is_dir()
    );
    assert!(!env.dir.path().join("cache/deeptree/backend.git").exists());
    assert!(env.dir.path().join("cache/deeptree/frontend.git").is_dir());
    let repos = env.json(&["repo", "ls", "--json"])["repos"].clone();
    assert_eq!(repos[0]["location"], "local");
    assert_eq!(repos[1]["location"], env.machine_location("vm"));
    let table = env.stdout(&["repo", "ls"]);
    assert!(table.contains(&env.machine_location("vm")), "{table}");
    let status = env.json(&["status", "--json"]);
    assert_eq!(status["repos"][1]["location"], env.machine_location("vm"));
    assert_eq!(status["repos"][1]["cached"], true);
    assert_eq!(status["repos"][1]["commit"]["subject"], "init");
}

#[test]
fn a_machine_that_cannot_be_reached_registers_nothing() {
    let env = Env::with_repos(&[]);
    env.octok(&[
        "repo",
        "add",
        &env.remote("backend"),
        "--location",
        "unreachable",
    ])
    .assert()
    .code(7)
    .stderr(predicate::str::contains(
        "cannot run commands on `unreachable` over ssh",
    ))
    .stderr(predicate::str::contains(
        "octok set-ssh-passphrase unreachable",
    ));

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([])
    );
}

#[test]
fn a_workspace_spans_this_machine_and_the_other_one() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();
    let local = env.workspace_dir("feat-login");
    let remote = env
        .machine_root()
        .join("workspaces/deeptree/feat-login/backend");

    env.octok(&["branch", "feat/login"])
        .assert()
        .success()
        .stdout(format!("{}\n", local.display()))
        .stderr(predicate::str::contains(format!(
            "backend: vm:{}",
            remote.display()
        )));

    assert_eq!(
        git(&local.join("frontend"), &["branch", "--show-current"]),
        "feat/login"
    );
    assert_eq!(git(&remote, &["branch", "--show-current"]), "feat/login");
    assert!(!local.join("backend").exists());
    let status = env.json(&["status", "--json"]);
    let checkouts = &status["workspaces"][0]["checkouts"];
    assert_eq!(
        checkouts[0]["path"],
        local.join("frontend").to_str().unwrap()
    );
    assert_eq!(checkouts[1]["branch"], "feat/login");
    assert_eq!(
        checkouts[1]["path"],
        format!("vm:{}", remote.display()).as_str()
    );
    let text = env.stdout(&["status"]);
    assert!(text.contains(&format!("vm:{}", remote.display())), "{text}");
    assert_eq!(
        env.stdout(&["checkout", "feat/login"]),
        format!("{}\n", local.display())
    );
}

#[test]
fn pull_and_rm_reach_the_worktrees_on_the_other_machine() {
    let env = Env::with_repos(&[]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();
    let remote = env
        .machine_root()
        .join("workspaces/deeptree/feat-login/backend");
    git(&remote, &["push", "--quiet"]);
    let origin = env.remote_dir("backend");
    git(&origin, &["checkout", "--quiet", "feat/login"]);
    commit(&origin, "more");

    env.octok(&["pull", "feat/login"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backend  updated"));
    assert_eq!(git(&remote, &["log", "-1", "--format=%s"]), "more");

    fs::write(remote.join("notes.txt"), "unsaved").unwrap();
    env.octok(&["rm", "feat/login"])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("backend: uncommitted changes"));
    assert!(remote.is_dir());

    env.octok(&["rm", "--force", "feat/login"])
        .assert()
        .success();
    assert!(!remote.parent().unwrap().exists());
    assert_eq!(
        git(
            &env.machine_root().join("cache/deeptree/backend.git"),
            &["branch", "--list"]
        ),
        ""
    );
    env.octok(&["pull", "feat/login"]).assert().code(3);
}

#[test]
fn a_repo_changes_machine_only_while_no_workspace_uses_it() {
    let env = Env::with_repos(&["backend"]);
    let local_cache = env.dir.path().join("cache/deeptree/backend.git");
    let remote_cache = env.machine_root().join("cache/deeptree/backend.git");
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["repo", "set", "backend", "--location", "vm"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains(
            "repo `backend` is used by workspaces: feat-login",
        ));
    assert!(local_cache.is_dir());
    assert!(!remote_cache.exists());

    env.octok(&["rm", "feat/login"]).assert().success();
    env.octok(&["repo", "set", "backend", "--location", "vm"])
        .assert()
        .success();
    assert!(remote_cache.is_dir());
    assert!(!local_cache.exists());
    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"][0]["location"],
        env.machine_location("vm")
    );

    env.octok(&["repo", "set", "backend", "--no-location"])
        .assert()
        .success();
    assert!(local_cache.is_dir());
    assert!(!remote_cache.exists());
    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"][0]["location"],
        "local"
    );
}

#[test]
fn project_set_location_moves_the_repos_that_follow_the_project() {
    let env = Env::with_repos(&["backend"]);
    env.octok(&[
        "repo",
        "add",
        &env.remote("frontend"),
        "--location",
        "local",
    ])
    .assert()
    .success();
    let cache = |root: &Path, repo: &str| root.join(format!("cache/deeptree/{repo}.git"));
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["project", "set", "--location", "vm"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("still has workspaces: feat-login"));

    env.octok(&["rm", "feat/login"]).assert().success();
    env.octok(&["project", "set", "--location", "vm"])
        .assert()
        .success();
    assert!(cache(&env.machine_root(), "backend").is_dir());
    assert!(!cache(env.dir.path(), "backend").exists());
    assert!(cache(env.dir.path(), "frontend").is_dir());
    assert!(!cache(&env.machine_root(), "frontend").exists());

    env.octok(&["project", "set", "--no-location"])
        .assert()
        .success();
    assert!(cache(env.dir.path(), "backend").is_dir());
    assert!(!cache(&env.machine_root(), "backend").exists());
}

#[test]
fn project_rm_deletes_the_caches_on_every_machine() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();

    env.octok(&["project", "rm", "deeptree"]).assert().success();

    assert!(!env.machine_root().join("cache/deeptree").exists());
    assert!(!env.dir.path().join("cache/deeptree").exists());
}

#[test]
fn project_rm_force_deletes_the_workspaces_and_leaves_nothing_on_the_other_machine() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["project", "rm", "--force", "deeptree"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed project `deeptree`"));

    assert!(env.project_dir_entries().is_empty());
    assert!(!env.dir.path().join("cache/deeptree").exists());
    assert!(!env.workspace_dir("feat-login").exists());
    assert!(!env.machine_root().exists());
}

#[test]
fn project_rm_keeps_the_folders_of_other_projects_on_the_other_machine() {
    let env = Env::new();
    for project in ["deeptree", "side"] {
        env.octok(&[
            "project",
            "init",
            project,
            "--base",
            "main",
            "--location",
            "vm",
        ])
        .assert()
        .success();
        env.octok(&["repo", "add", &env.remote(project)])
            .assert()
            .success();
    }

    env.octok(&["project", "rm", "side"]).assert().success();

    assert!(!env.machine_root().join("cache/side").exists());
    assert!(
        env.machine_root()
            .join("cache/deeptree/deeptree.git")
            .is_dir()
    );
}

#[test]
fn set_ssh_passphrase_hands_the_keys_of_the_hosts_to_ssh_add() {
    let env = Env::with_repos(&[]);
    let key = env.machine_file("id_test");
    fs::write(&key, "a private key").unwrap();
    fs::write(
        env.machine_file("ssh.config"),
        format!("hostname 10.0.0.5\nidentityfile {}\n", key.display()),
    )
    .unwrap();
    let added = || fs::read_to_string(env.machine_file("ssh-add.log")).unwrap_or_default();

    env.octok(&["set-ssh-passphrase"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no repo on another machine"));
    assert_eq!(added(), "");

    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();
    env.octok(&["set-ssh-passphrase"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "vm: added {} to the ssh agent",
            key.display()
        )));
    assert!(added().trim_end().ends_with(key.to_str().unwrap()));
    assert!(
        fs::read_to_string(env.machine_file("ssh.log"))
            .unwrap()
            .contains("-G vm\n")
    );

    env.octok(&["set-ssh-passphrase", "other-host"])
        .assert()
        .success();
    assert!(
        fs::read_to_string(env.machine_file("ssh.log"))
            .unwrap()
            .contains("-G other-host\n")
    );
}

#[test]
fn a_workspace_can_live_whole_on_one_machine() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();
    let here = env.workspace_dir("feat-here");
    let there = env.machine_root().join("workspaces/deeptree/feat-there");
    let local_cache = |repo: &str| env.dir.path().join(format!("cache/deeptree/{repo}.git"));
    let remote_cache = |repo: &str| {
        env.machine_root()
            .join(format!("cache/deeptree/{repo}.git"))
    };

    env.octok(&["branch", "feat/here", "--location", "local"])
        .assert()
        .success()
        .stdout(format!("{}\n", here.display()));
    env.octok(&["checkout", "-b", "feat/there", "--location", "vm"])
        .assert()
        .success()
        .stdout(format!("vm:{}\n", there.display()));

    for repo in ["frontend", "backend"] {
        let branch = ["branch", "--show-current"];
        assert_eq!(git(&here.join(repo), &branch), "feat/here");
        assert_eq!(git(&there.join(repo), &branch), "feat/there");
    }
    let project_file = env.read("projects/deeptree.yaml");
    assert!(
        project_file.contains("workspaces:\n  feat-here:\n    location: local\n  feat-there:\n"),
        "{project_file}"
    );
    let status = env.json(&["status", "--json"]);
    let workspaces = status["workspaces"].as_array().unwrap();
    assert_eq!(workspaces.len(), 2);
    assert_eq!(workspaces[0]["path"], here.to_str().unwrap());
    assert_eq!(
        workspaces[1]["path"],
        format!("vm:{}", there.display()).as_str()
    );
    for workspace in workspaces {
        assert_eq!(workspace["checkouts"].as_array().unwrap().len(), 2);
    }

    env.octok(&["branch", "feat/here", "--location", "vm"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains(format!(
            "workspace `feat-here` already exists at {}",
            here.display()
        )));

    env.octok(&["rm", "feat/here"]).assert().success();
    assert!(!here.exists());
    assert!(!local_cache("backend").exists());
    assert!(local_cache("frontend").is_dir());
    env.octok(&["rm", "feat/there"]).assert().success();
    assert!(!there.exists());
    assert!(!remote_cache("frontend").exists());
    assert!(remote_cache("backend").is_dir());
    let project_file = env.read("projects/deeptree.yaml");
    assert!(!project_file.contains("workspaces"), "{project_file}");
}

#[test]
fn a_branch_that_fails_forgets_the_location_it_asked_for() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    let url = env.remote_with_branch("frontend", "feat/taken");
    env.octok(&["repo", "add", &url]).assert().success();

    env.octok(&["branch", "feat/taken", "--location", "vm"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("already exists on the remote"));

    let project_file = env.read("projects/deeptree.yaml");
    assert!(!project_file.contains("workspaces"), "{project_file}");
    assert!(
        !env.machine_root()
            .join("cache/deeptree/frontend.git")
            .exists()
    );
}

#[test]
fn repo_add_at_a_terminal_asks_for_what_the_flags_leave_out() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    let url = env.remote_with_branch("frontend", "develop");
    let origin = env.remote_dir("frontend");
    git(&origin, &["branch", "Staging"]);
    for year in 2021..=2026 {
        commit_on_new_branch(
            &origin,
            &format!("feat/{year}"),
            &format!("{year}-06-01T00:00:00Z"),
        );
    }
    git(&origin, &["checkout", "--quiet", "main"]);

    let mut terminal = env.terminal(&["repo", "add", &url]);
    answer(&mut terminal, "Alias [frontend]: ", "fe");
    answer(&mut terminal, "Location [local, project default]: ", "");
    terminal
        .exp_string(
            "Branches on the remote:\r\n  main (default)\r\n  Staging\r\n  develop\r\n  \
             feat/2026\r\n  feat/2025\r\n  feat/2024\r\n  feat/2023\r\n  feat/2022\r\n  \
             ... and 1 more\r\n",
        )
        .unwrap();
    answer(
        &mut terminal,
        "Base branch [main, project default]: ",
        "develop",
    );
    let printed = finish(terminal);

    assert!(
        printed.contains("added `frontend (fe)` to deeptree"),
        "{printed}"
    );
    let repos: serde_json::Value =
        serde_json::from_str(&env.stdout(&["repo", "ls", "--json"])).unwrap();
    assert_eq!(repos["repos"][0]["key"], "frontend");
    assert_eq!(repos["repos"][0]["alias"], "fe");
    assert_eq!(repos["repos"][0]["base"], "develop");
    assert_eq!(repos["repos"][0]["location"], "local");
}

#[test]
fn repo_add_at_a_terminal_with_enter_on_everything_saves_what_the_flags_would() {
    let asked = Env::new();
    let plain = Env::new();
    for env in [&asked, &plain] {
        env.octok(&["project", "init", "deeptree", "--base", "main"])
            .assert()
            .success();
    }
    let url = asked.remote("frontend");

    let mut terminal = asked.terminal(&["repo", "add", &url]);
    for question in ["Alias [frontend]: ", "Location [", "Base branch ["] {
        answer(&mut terminal, question, "");
    }
    finish(terminal);
    plain.octok(&["repo", "add", &url]).assert().success();

    assert_eq!(
        asked.read("projects/deeptree.yaml"),
        plain.read("projects/deeptree.yaml")
    );
}

#[test]
fn repo_add_at_a_terminal_asks_again_after_an_answer_it_cannot_use() {
    let env = Env::with_repos(&["frontend"]);
    let key = env.describe_machine_to_ssh();
    let url = env.remote("backend");
    env.write_ssh_config("Host vm other-vm\nHost *.internal\n");

    let mut terminal = env.terminal(&["repo", "add", &url]);
    answer(&mut terminal, "Alias [backend]: ", "frontend");
    terminal
        .exp_string("alias `frontend` is already used")
        .unwrap();
    answer(&mut terminal, "Alias [backend]: ", "backend");
    terminal
        .exp_string("Machines in ~/.ssh/config:\r\n  vm\r\n  other-vm\r\n")
        .unwrap();
    answer(
        &mut terminal,
        "Location [local, project default]: ",
        "unreachable",
    );
    terminal
        .exp_string("cannot run commands on `unreachable` over ssh")
        .unwrap();
    answer(&mut terminal, "Location [local, project default]: ", "vm");
    terminal
        .exp_string(&format!(
            "ssh reaches `vm` as ubuntu@10.0.0.5, port 2222, key {}",
            key.display()
        ))
        .unwrap();
    answer(
        &mut terminal,
        "Base branch [main, project default]: ",
        "not a branch",
    );
    terminal
        .exp_string("invalid branch `not a branch`")
        .unwrap();
    answer(&mut terminal, "Base branch [main, project default]: ", "");
    let printed = finish(terminal);

    assert!(printed.contains("added `backend` to deeptree"), "{printed}");
    assert!(
        env.machine_root()
            .join("cache/deeptree/backend.git")
            .is_dir()
    );
    let log = fs::read_to_string(env.machine_file("ssh.log")).unwrap();
    assert!(log.contains("for-each-ref"), "{log}");
}

#[test]
fn repo_add_at_a_terminal_never_asks_for_the_name() {
    let env = Env::with_repos(&["frontend"]);
    let url = env.remote_url("frontend");

    let mut terminal = env.terminal(&["repo", "add", &url]);
    let printed = terminal.exp_eof().unwrap();
    assert!(
        printed.contains("repo `frontend` is already registered"),
        "{printed}"
    );
    assert!(printed.contains("--name"), "{printed}");
    assert!(!printed.contains("]: "), "{printed}");

    let mut terminal = env.terminal(&["repo", "add", &url, "--name", "second"]);
    for question in ["Alias [second]: ", "Location [", "Base branch ["] {
        answer(&mut terminal, question, "");
    }
    let printed = finish(terminal);
    assert!(printed.contains("added `second` to deeptree"), "{printed}");
}

#[test]
fn repo_add_at_a_terminal_with_every_flag_asks_nothing() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    let url = env.remote("frontend");

    let terminal = env.terminal(&[
        "repo",
        "add",
        &url,
        "--name",
        "dashboard",
        "--alias",
        "fe",
        "--base",
        "main",
        "--location",
        "local",
    ]);
    let printed = finish(terminal);

    assert!(!printed.contains("]: "), "{printed}");
    assert!(
        printed.contains("added `dashboard (fe)` to deeptree"),
        "{printed}"
    );
}

#[test]
fn repo_add_at_a_terminal_asks_before_using_a_base_missing_on_the_remote() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "main"])
        .assert()
        .success();
    let url = env.remote("frontend");
    let flags = ["--name", "frontend", "--alias", "fe", "--location", "local"];
    let add = |base: &str| {
        let mut args = vec!["repo", "add", url.as_str(), "--base", base];
        args.extend(flags);
        env.terminal(&args)
    };

    let mut terminal = add("develop");
    answer(&mut terminal, "Use it as base anyway? [y/N] ", "n");
    terminal
        .exp_string("has no branch `develop` on its remote")
        .unwrap();
    terminal.exp_eof().unwrap();
    assert!(!env.read("projects/deeptree.yaml").contains("frontend"));

    let mut terminal = add("develop");
    answer(&mut terminal, "Use it as base anyway? [y/N] ", "y");
    finish(terminal);
    assert!(env.read("projects/deeptree.yaml").contains("base: develop"));
}

#[test]
fn project_init_at_a_terminal_asks_for_the_base_and_the_location() {
    let env = Env::new();
    env.describe_machine_to_ssh();

    env.write_ssh_config("Host vm\n");

    let mut terminal = env.terminal(&["project", "init", "deeptree"]);
    answer(&mut terminal, "Base branch [staging]: ", "not a branch");
    terminal
        .exp_string("invalid branch `not a branch`")
        .unwrap();
    answer(&mut terminal, "Base branch [staging]: ", "");
    terminal
        .exp_string("Machines in ~/.ssh/config:\r\n  vm\r\n")
        .unwrap();
    answer(&mut terminal, "Location [local]: ", "vm");
    terminal
        .exp_string("ssh reaches `vm` as ubuntu@10.0.0.5")
        .unwrap();
    let printed = finish(terminal);

    assert!(
        printed.contains("created project `deeptree` (base: staging)"),
        "{printed}"
    );
    assert!(
        printed.contains(&format!("its repos live at {}", env.machine_location("vm"))),
        "{printed}"
    );
    assert!(env.machine_root().is_dir());
}

#[test]
fn project_init_at_a_terminal_with_every_flag_asks_nothing() {
    let env = Env::new();

    let terminal = env.terminal(&[
        "project",
        "init",
        "deeptree",
        "--base",
        "main",
        "--location",
        "local",
    ]);
    let printed = finish(terminal);

    assert!(!printed.contains("]: "), "{printed}");
    assert!(printed.contains("(base: main)"), "{printed}");
}

#[test]
fn project_init_without_a_terminal_uses_staging_and_this_machine() {
    let env = Env::new();

    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "created project `deeptree` (base: staging)",
        ));

    let project = env.read("projects/deeptree.yaml");
    assert!(project.contains("base: staging"), "{project}");
    assert!(!project.contains("location"), "{project}");
}

/// Prints `ready` once it has started a `sleep` and written its pid to `sleep.pid`.
const SLEEP_IN_BACKGROUND: &str = "sleep 300 & echo $! > sleep.pid; echo ready; wait";

/// An `octok up` left running, with what it prints on stdout.
struct Up {
    octok: Child,
    stdout: BufReader<ChildStdout>,
}

impl Up {
    fn start(env: &Env) -> Self {
        Self::start_with(env, &["up", "--branch", "feat/login"])
    }

    fn start_with(env: &Env, args: &[&str]) -> Self {
        let mut octok = env
            .octok(args)
            // Not the terminal of whoever runs the tests.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = BufReader::new(octok.stdout.take().unwrap());
        Self { octok, stdout }
    }

    fn wait_for_line(&mut self, wanted: &str) {
        let mut line = String::new();
        while line.trim_end() != wanted {
            line.clear();
            let read = self.stdout.read_line(&mut line).unwrap();
            assert_ne!(read, 0, "octok ended before printing {wanted:?}");
        }
    }

    fn signal(&self, signal: &str) {
        let pid = self.octok.id().to_string();
        let sent = Command::new("kill").args([signal, &pid]).status().unwrap();
        assert!(sent.success());
    }

    /// Waits for `octok` to end and returns its exit code and what it printed on stderr.
    fn finish(self) -> (Option<i32>, String) {
        let output = self.octok.wait_with_output().unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        (output.status.code(), stderr)
    }
}

/// The workspace `feat-login` with one repo that sleeps once `octok up` has started it.
struct Sleeping {
    env: Env,
    repo: &'static str,
    /// Holds the pid of the `sleep` that the run command started.
    pid_file: PathBuf,
}

impl Sleeping {
    /// The repo `frontend`, on this machine.
    fn here() -> Self {
        let env = Env::with_repos(&["frontend"]);
        let pid_file = env.workspace_dir("feat-login").join("frontend/sleep.pid");
        Self::new(env, "frontend", pid_file)
    }

    /// The repo `backend`, on the other machine.
    fn on_the_other_machine() -> Self {
        let env = Env::with_repos(&[]);
        env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
            .assert()
            .success();
        let checkouts = env.machine_root().join("workspaces/deeptree/feat-login");
        Self::new(env, "backend", checkouts.join("backend/sleep.pid"))
    }

    fn new(env: Env, repo: &'static str, pid_file: PathBuf) -> Self {
        env.octok(&["repo", "set", repo, "--run", SLEEP_IN_BACKGROUND])
            .assert()
            .success();
        env.octok(&["branch", "feat/login"]).assert().success();
        Self {
            env,
            repo,
            pid_file,
        }
    }

    /// Starts `octok up` and waits for the repo to say `ready`.
    fn up(&self) -> Up {
        let mut up = Up::start(&self.env);
        up.wait_for_line(&format!("{} | ready", self.repo));
        up
    }

    fn is_running(&self) -> bool {
        let pid = fs::read_to_string(&self.pid_file).unwrap();
        Command::new("kill")
            .args(["-0", pid.trim()])
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    }

    /// The `sleep` is gone, at the latest a moment from now.
    fn assert_stopped(&self) {
        self.assert_stopped_within(Duration::from_secs(5));
    }

    fn assert_stopped_within(&self, limit: Duration) {
        let deadline = Instant::now() + limit;
        while self.is_running() {
            assert!(Instant::now() < deadline, "the sleep is still running");
            thread::sleep(Duration::from_millis(50));
        }
    }

    /// `signal` sent to `octok up` stops the repo, and octok says so.
    fn assert_stopped_by(&self, signal: &str) {
        let up = self.up();
        up.signal(signal);
        let (code, stderr) = up.finish();

        assert_eq!(code, Some(130), "{signal}: {stderr}");
        let stopped = format!("stopping...\n{} | stopped\n", self.repo);
        assert!(stderr.contains(&stopped), "{signal}: {stderr}");
        self.assert_stopped();
    }

    /// The repo stops by itself when `octok up` is killed, with no chance to stop it.
    fn assert_stopped_when_octok_is_killed(&self) {
        let up = self.up();
        up.signal("-KILL");
        up.finish();

        self.assert_stopped();
    }
}

#[test]
fn repo_set_changes_the_setup_and_run_commands() {
    let env = Env::with_repos(&["frontend"]);

    env.octok(&[
        "repo",
        "set",
        "frontend",
        "--setup",
        "npm ci",
        "--run",
        "npm run dev -- --port 3000",
    ])
    .assert()
    .success();
    let yaml = env.read("projects/deeptree.yaml");
    assert!(yaml.contains("    setup: npm ci\n"), "{yaml}");
    assert!(
        yaml.contains("    run: npm run dev -- --port 3000\n"),
        "{yaml}"
    );

    env.octok(&["repo", "set", "frontend", "--no-setup", "--no-run"])
        .assert()
        .success();
    let yaml = env.read("projects/deeptree.yaml");
    assert!(!yaml.contains("setup"), "{yaml}");
    assert!(!yaml.contains("run"), "{yaml}");

    env.octok(&["repo", "set", "frontend", "--run", "x", "--no-run"])
        .assert()
        .code(2);
}

#[test]
fn repo_set_with_a_branch_changes_the_run_command_of_that_workspace_only() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&[
        "repo",
        "set",
        "frontend",
        "--alias",
        "web",
        "--run",
        "echo 3000",
    ])
    .assert()
    .success();
    env.octok(&["branch", "feat/login"]).assert().success();
    env.octok(&["branch", "feat/other"]).assert().success();
    let up = |branch: &str| env.stdout(&["up", "--branch", branch]);

    env.octok(&[
        "repo",
        "set",
        "web",
        "--run",
        "echo 3001",
        "--branch",
        "feat/login",
    ])
    .assert()
    .success()
    .stdout("updated `frontend` in workspace `feat-login` of deeptree\n");
    let yaml = env.read("projects/deeptree.yaml");
    assert!(
        yaml.contains(
            "workspaces:\n  feat-login:\n    repos:\n      frontend:\n        run: echo 3001\n"
        ),
        "{yaml}"
    );
    assert!(up("feat/login").contains("web | 3001\n"));
    assert!(up("feat/other").contains("web | 3000\n"));

    env.octok(&["repo", "set", "web", "--no-run", "--branch", "feat/login"])
        .assert()
        .success();
    assert!(up("feat/login").contains("web | 3000\n"));
    assert!(!env.read("projects/deeptree.yaml").contains("workspaces"));

    env.octok(&["repo", "set", "web", "--branch", "feat/login"])
        .assert()
        .code(2);
    env.octok(&[
        "repo",
        "set",
        "web",
        "--run",
        "x",
        "--alias",
        "w",
        "--branch",
        "feat/login",
    ])
    .assert()
    .code(2);
}

#[test]
fn up_prefixes_every_line_with_its_repo_and_ends_with_the_commands() {
    let env = Env::with_repos(&["frontend", "backend", "docs"]);
    env.octok(&[
        "repo",
        "set",
        "frontend",
        "--alias",
        "web",
        "--run",
        "echo hello",
    ])
    .assert()
    .success();
    env.octok(&["repo", "set", "backend", "--run", "echo hi; echo oops >&2"])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["up", "--branch", "feat/login"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("web     | $ echo hello\n")
                .and(predicate::str::contains("web     | hello\n"))
                .and(predicate::str::contains("backend | hi\n"))
                .and(predicate::str::contains("backend | oops\n"))
                .and(predicate::str::contains("docs").not()),
        )
        .stderr(
            predicate::str::contains("web     | ended (exit status: 0)\n").and(
                predicate::str::contains("backend | ended (exit status: 0)\n"),
            ),
        );
}

#[test]
fn up_starts_only_the_repos_it_is_given_after_their_setup() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&[
        "repo",
        "set",
        "frontend",
        "--alias",
        "web",
        "--setup",
        "echo installing",
        "--run",
        "echo hello",
    ])
    .assert()
    .success();
    env.octok(&["repo", "set", "backend", "--run", "echo hi"])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();

    let first = env.stdout(&["up", "web", "--branch", "feat/login"]);
    assert_eq!(
        first,
        "web | $ echo installing\nweb | installing\nweb | $ echo hello\nweb | hello\n"
    );
    let second = env.stdout(&["up", "web", "--branch", "feat/login"]);
    assert_eq!(second, "web | $ echo hello\nweb | hello\n");

    env.octok(&["up", "nope", "--branch", "feat/login"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("nope"));
}

#[test]
fn up_uses_the_workspace_of_the_current_directory_and_fails_with_a_command() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "set", "frontend", "--run", "exit 3"])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["up"])
        .current_dir(env.workspace_dir("feat-login").join("frontend"))
        .assert()
        .code(1)
        .stderr("frontend | ended (exit status: 3)\n");
    env.octok(&["up"])
        .current_dir(env.dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("octok up --branch <branch>"));
}

#[test]
fn up_without_a_run_command_says_how_to_set_one() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["up", "--branch", "feat/login"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("octok repo set <repo> --run"));
}

#[test]
fn up_stops_every_command_on_ctrl_c_and_when_it_is_told_to_end() {
    let sleeping = Sleeping::here();

    for signal in ["-INT", "-TERM", "-HUP"] {
        sleeping.assert_stopped_by(signal);
    }
}

#[test]
fn up_stops_every_command_on_ctrl_c_at_a_terminal() {
    let sleeping = Sleeping::here();

    let mut terminal = sleeping.env.terminal(&["up", "--branch", "feat/login"]);
    terminal.exp_string("frontend | ready").unwrap();
    terminal.send_control('c').unwrap();
    let printed = terminal.exp_eof().unwrap();
    let status = terminal.process().wait().unwrap();

    assert!(matches!(status, WaitStatus::Exited(_, 130)), "{status:?}");
    assert!(printed.contains("frontend | stopped"), "{printed}");
    sleeping.assert_stopped();
}

#[test]
fn the_commands_stop_when_octok_is_killed() {
    Sleeping::here().assert_stopped_when_octok_is_killed();
}

#[test]
fn up_runs_the_commands_of_a_repo_on_the_other_machine_in_its_checkout() {
    let env = Env::with_repos(&[]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();
    env.octok(&[
        "repo",
        "set",
        "backend",
        "--setup",
        r#"echo "it's set up""#,
        "--run",
        "pwd -P; echo done",
    ])
    .assert()
    .success();
    env.octok(&["branch", "feat/login"]).assert().success();
    let checkout = env
        .machine_root()
        .join("workspaces/deeptree/feat-login/backend");

    let printed = env.stdout(&["up", "--branch", "feat/login"]);

    assert_eq!(
        printed,
        format!(
            "backend | $ echo \"it's set up\"\nbackend | it's set up\nbackend | $ pwd -P; echo done\nbackend | {}\nbackend | done\n",
            checkout.display()
        )
    );
    let again = env.stdout(&["up", "--branch", "feat/login"]);
    assert!(!again.contains("set up"), "the setup runs once: {again}");
    let ssh_log = fs::read_to_string(env.machine_file("ssh.log")).unwrap();
    assert!(ssh_log.contains("ServerAliveInterval=5"), "{ssh_log}");
}

#[test]
fn up_stops_the_commands_on_the_other_machine_on_ctrl_c() {
    Sleeping::on_the_other_machine().assert_stopped_by("-INT");
}

#[test]
fn the_commands_on_the_other_machine_stop_when_octok_is_killed() {
    Sleeping::on_the_other_machine().assert_stopped_when_octok_is_killed();
}

#[test]
fn the_commands_on_the_other_machine_stop_when_octok_goes_silent() {
    let sleeping = Sleeping::on_the_other_machine();

    let up = sleeping.up();
    // Frozen, octok keeps the connection open and says nothing: what a network that died
    // without closing the connection looks like from the other machine.
    up.signal("-STOP");
    thread::sleep(Duration::from_secs(10));
    assert!(
        sleeping.is_running(),
        "stopped before 30 seconds of silence"
    );

    sleeping.assert_stopped_within(Duration::from_secs(40));
    up.signal("-KILL");
    up.finish();
}

const APP_PY: &str = "import os\n\n\ndef main():\n    return os.name\n\n\nrun(port=8000)\n";

/// A project whose repo `backend` tracks `app.py`, and the workspace `feat-login`.
/// `location` is where the repo lives: `None` is this machine.
fn env_with_app(location: Option<&str>) -> Env {
    let env = Env::with_repos(&[]);
    let url = env.remote("backend");
    commit_file(&env.remote_dir("backend"), "app.py", APP_PY);
    let mut add = vec!["repo", "add", &url];
    add.extend(
        location
            .iter()
            .flat_map(|location| ["--location", location]),
    );
    env.octok(&add).assert().success();
    env.octok(&["branch", "feat/login"]).assert().success();
    env
}

fn commit_file(repo: &Path, file: &str, content: &str) {
    fs::write(repo.join(file), content).unwrap();
    git(repo, &["add", "--all"]);
    commit(repo, "change a file");
}

/// Dev changes made in the checkout of `feat-login` are saved, shown, and found again in
/// a new workspace. `workspaces` is the folder of the workspaces of the project.
fn assert_dev_changes_reach_a_new_workspace(env: &Env, workspaces: &Path) {
    let login = workspaces.join("feat-login/backend");
    fs::write(
        login.join("app.py"),
        APP_PY.replace("port=8000", "port=9005"),
    )
    .unwrap();
    fs::write(login.join(".env"), "TOKEN=secret\n").unwrap();

    env.octok(&["dev", "save", "--file", ".env", "--branch", "feat/login"])
        .assert()
        .success()
        .stdout("backend\n  changed  app.py\n  new      .env\n");
    env.octok(&["dev", "show"]).assert().success().stdout(
        predicate::str::contains("  changed  app.py\n  new      .env\n")
            .and(predicate::str::contains(
                "-run(port=8000)\n+run(port=9005)\n",
            ))
            .and(predicate::str::contains("secret").not()),
    );
    assert_eq!(git(&login, &["status", "--porcelain"]), "");

    env.octok(&["branch", "feat/other"]).assert().success();
    let other = workspaces.join("feat-other/backend");
    let app = fs::read_to_string(other.join("app.py")).unwrap();
    assert!(app.contains("port=9005"), "{app}");
    assert_eq!(
        fs::read_to_string(other.join(".env")).unwrap(),
        "TOKEN=secret\n"
    );
    assert_eq!(git(&other, &["status", "--porcelain"]), "");
}

#[test]
fn dev_save_keeps_the_changes_of_a_checkout_for_every_new_workspace() {
    let env = env_with_app(None);

    assert_dev_changes_reach_a_new_workspace(&env, &env.dir.path().join("workspaces/deeptree"));
}

#[test]
fn dev_changes_work_for_a_repo_on_the_other_machine() {
    let env = env_with_app(Some("vm"));

    assert_dev_changes_reach_a_new_workspace(&env, &env.machine_root().join("workspaces/deeptree"));
}

#[test]
fn dev_save_uses_the_workspace_of_the_current_directory_and_says_when_nothing_is_saved() {
    let env = env_with_app(None);
    let checkout = env.workspace_dir("feat-login").join("backend");

    env.octok(&["dev", "save"])
        .current_dir(&checkout)
        .assert()
        .success()
        .stdout("backend\n  no dev changes\n");
    env.octok(&["dev", "save"])
        .current_dir(env.dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("octok dev save --branch <branch>"));
    env.octok(&["dev", "save", "--file", "nope.env"])
        .current_dir(&checkout)
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "no checkout has the file `nope.env`",
        ));
}

#[test]
fn branch_is_refused_when_the_dev_changes_no_longer_apply_unless_allowed() {
    let env = env_with_app(None);
    let login = env.workspace_dir("feat-login").join("backend");
    fs::write(
        login.join("app.py"),
        APP_PY.replace("port=8000", "port=9005"),
    )
    .unwrap();
    env.octok(&["dev", "save", "--branch", "feat/login"])
        .assert()
        .success();
    let upstream = APP_PY.replace("port=8000", "port=8080");
    commit_file(&env.remote_dir("backend"), "app.py", &upstream);

    env.octok(&["branch", "feat/other"])
        .assert()
        .code(5)
        .stdout("")
        .stderr(
            predicate::str::contains("the dev changes of `backend` no longer apply to: app.py")
                .and(predicate::str::contains("--allow-stale-dev")),
        );
    assert!(!env.workspace_dir("feat-other").exists());

    env.octok(&["checkout", "-b", "feat/other", "--allow-stale-dev"])
        .assert()
        .success();
    let other = env.workspace_dir("feat-other").join("backend");
    assert_eq!(fs::read_to_string(other.join("app.py")).unwrap(), upstream);
}

#[test]
fn the_saved_dev_changes_do_not_follow_a_repo_to_another_machine() {
    let env = env_with_app(None);
    let login = env.workspace_dir("feat-login").join("backend");
    fs::write(
        login.join("app.py"),
        APP_PY.replace("port=8000", "port=9005"),
    )
    .unwrap();
    env.octok(&["dev", "save", "--branch", "feat/login"])
        .assert()
        .success();
    env.octok(&["rm", "feat/login"]).assert().success();
    let saved = env.dir.path().join("dev/deeptree/backend");
    assert!(saved.is_dir());

    env.octok(&["repo", "set", "backend", "--location", "vm"])
        .assert()
        .success();

    assert!(!saved.exists());
    env.octok(&["dev", "show"])
        .assert()
        .success()
        .stdout("backend\n  no dev changes\n");
}

#[test]
fn dev_show_json_lists_the_saved_files_and_the_diff_of_each_repo() {
    let env = env_with_app(None);
    let login = env.workspace_dir("feat-login").join("backend");
    fs::write(
        login.join("app.py"),
        APP_PY.replace("port=8000", "port=9005"),
    )
    .unwrap();
    fs::write(login.join(".env"), "TOKEN=secret\n").unwrap();
    env.octok(&["dev", "save", "--file", ".env", "--branch", "feat/login"])
        .assert()
        .success();

    let shown = env.json(&["dev", "show", "--json"]);

    assert_eq!(shown["schema"], 1);
    assert_eq!(shown["project"], "deeptree");
    let backend = &shown["repos"][0];
    assert_eq!(backend["key"], "backend");
    assert_eq!(backend["alias"], serde_json::Value::Null);
    assert_eq!(backend["changed"], serde_json::json!(["app.py"]));
    assert_eq!(backend["new"], serde_json::json!([".env"]));
    let diff = backend["diff"].as_str().unwrap();
    assert!(diff.contains("-run(port=8000)\n+run(port=9005)"), "{diff}");
    assert!(!shown.to_string().contains("secret"), "{shown}");
}

fn current_branch(checkout: &Path) -> String {
    git(checkout, &["branch", "--show-current"])
}

#[test]
fn pr_review_puts_each_repo_on_its_pull_request_or_branch_and_the_others_on_the_base() {
    let env = Env::with_a_pull_request();
    let path = env.workspace_dir("pr-backend-12");

    env.octok(&[
        "pr",
        "review",
        "--branch",
        "agent=fix/prompt",
        "--pr",
        "backend=12",
    ])
    .assert()
    .success()
    .stdout(format!("{}\n", path.display()))
    .stderr(
        "backend: pull request #12 is the branch `feat/login`\n\
         created workspace `pr-backend-12` in deeptree\n  \
         branch from the remote: backend, agent\n  \
         new branch from the base: frontend\n",
    );

    assert_eq!(current_branch(&path.join("backend")), "feat/login");
    assert_eq!(current_branch(&path.join("agent")), "fix/prompt");
    assert_eq!(current_branch(&path.join("frontend")), "pr-backend-12");
    assert_eq!(
        git(&path.join("backend"), &["rev-parse", "HEAD"]),
        git(&env.remote_dir("backend"), &["rev-parse", "feat/login"])
    );
    env.octok(&["pull", "pr-backend-12"])
        .assert()
        .success()
        .stdout(predicate::str::contains("backend   up to date"));
    env.octok(&["rm", "pr-backend-12"]).assert().success();
    assert!(!path.exists());
}

#[test]
fn pr_review_names_the_workspace_after_the_pull_request_the_branch_or_the_given_name() {
    let env = Env::with_a_pull_request();
    env.octok(&["repo", "set", "backend", "--alias", "api"])
        .assert()
        .success();
    let path = |name: &str| format!("{}\n", env.workspace_dir(name).display());

    assert_eq!(
        env.stdout(&["pr", "review", "--pr", "api=#12"]),
        path("pr-api-12")
    );
    env.octok(&["rm", "pr-api-12"]).assert().success();
    assert_eq!(
        env.stdout(&["pr", "review", "--branch", "agent=fix/prompt"]),
        path("fix-prompt")
    );
    assert_eq!(
        env.stdout(&[
            "pr",
            "review",
            "--pr",
            "backend=12",
            "--name",
            "review/login"
        ]),
        path("review-login")
    );

    let named = env.workspace_dir("review-login");
    assert_eq!(current_branch(&named.join("backend")), "feat/login");
    assert_eq!(current_branch(&named.join("frontend")), "review/login");
    assert_eq!(
        current_branch(&env.workspace_dir("fix-prompt").join("frontend")),
        "fix/prompt"
    );
}

#[test]
fn pr_review_refuses_what_it_cannot_review_and_creates_nothing() {
    let env = Env::with_a_pull_request();
    env.pull_request("backend", 13, "patch-1", true);
    let refused = |code: i32, args: &[&str], message: &str| {
        env.octok(args)
            .assert()
            .code(code)
            .stdout("")
            .stderr(predicate::str::contains(message));
    };

    refused(
        2,
        &["pr", "review"],
        "error: nothing to review: choose a pull request or a branch for at least one repo",
    );
    refused(
        1,
        &["pr", "review", "--pr", "backend=13"],
        "error: pull request #13 of `backend` comes from a fork",
    );
    refused(
        1,
        &["pr", "review", "--pr", "backend=99"],
        "Could not resolve to a PullRequest with the number of 99",
    );
    refused(
        2,
        &[
            "pr",
            "review",
            "--pr",
            "backend=12",
            "--branch",
            "backend=x",
        ],
        "error: repo `backend` is given more than once",
    );
    refused(
        3,
        &["pr", "review", "--pr", "billing=12"],
        "error: repo `billing` is not registered",
    );
    refused(
        3,
        &["pr", "review", "--branch", "frontend=feat/login"],
        "error: repo `frontend` has no branch `feat/login` on its remote",
    );
    let misuse = |args: &[&str], message: &str| refused(2, args, message);
    let number = "expected `<repo>=<number>`, e.g. `backend=12`";
    misuse(&["pr", "review", "--pr", "backend"], number);
    misuse(&["pr", "review", "--pr", "backend=twelve"], number);
    misuse(
        &["pr", "review", "--branch", "agent="],
        "expected `<repo>=<branch>`, e.g. `agent=fix/prompt`",
    );

    assert!(!env.dir.path().join("workspaces").exists());
}

#[test]
fn pr_review_checks_every_flag_before_it_asks_gh_anything() {
    let env = Env::with_a_pull_request();

    env.octok(&["pr", "review", "--pr", "backend=12", "--branch", "nope=x"])
        .assert()
        .code(3)
        .stderr("error: repo `nope` is not registered\n");
}

#[test]
fn pr_review_remembers_what_a_workspace_reviews() {
    let env = Env::with_a_pull_request();
    env.pull_request("agent", 7, "fix/prompt", false);
    let path = env.workspace_dir("pr-backend-12");
    env.octok(&["pr", "review", "--pr", "backend=12"])
        .assert()
        .success();
    let project_file = env.read("projects/deeptree.yaml");
    assert!(
        project_file.contains(
            "workspaces:\n  pr-backend-12:\n    branch: pr-backend-12\n    repos:\n      \
             backend:\n        branch: feat/login\n"
        ),
        "{project_file}"
    );

    env.octok(&[
        "pr",
        "review",
        "--pr",
        "backend=12",
        "--pr",
        "agent=7",
        "--name",
        "pr-backend-12",
    ])
    .assert()
    .code(5)
    .stdout("")
    .stderr(predicate::str::contains(
        "error: workspace `pr-backend-12` already reviews other branches: backend=feat/login\n\
         remove it with `octok rm pr-backend-12`, or choose another name with `--name`\n",
    ));
    assert_eq!(current_branch(&path.join("agent")), "pr-backend-12");

    // Any command that completes the workspace puts back the branch it reviews.
    fs::remove_dir_all(path.join("backend")).unwrap();
    env.octok(&["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "backend   no worktree: add it with `octok checkout pr-backend-12`\n",
        ));
    env.octok(&["checkout", "pr-backend-12"])
        .assert()
        .success()
        .stderr(
            "completed workspace `pr-backend-12` in deeptree\n  \
             branch restored from the cache: backend\n",
        );
    assert_eq!(current_branch(&path.join("backend")), "feat/login");

    env.octok(&["rm", "pr-backend-12"]).assert().success();
    assert!(!env.read("projects/deeptree.yaml").contains("workspaces"));
}

#[test]
fn pr_review_refuses_a_branch_that_another_workspace_has() {
    let env = Env::with_a_pull_request();
    let own = env.workspace_dir("feat-login");
    env.octok(&["checkout", "feat/login"]).assert().success();

    env.octok(&["pr", "review", "--pr", "backend=12"])
        .assert()
        .code(5)
        .stderr(predicate::str::contains(format!(
            "error: branch `feat/login` of `backend` is already checked out in {}",
            // git names the folder by its real path.
            own.join("backend").canonicalize().unwrap().display()
        )));

    assert!(!env.workspace_dir("pr-backend-12").exists());
}

#[test]
fn pr_review_at_a_terminal_asks_what_to_take_for_each_repo() {
    let env = Env::with_a_pull_request();
    env.pull_request("backend", 13, "patch-1", true);

    let mut terminal = env.terminal(&["pr", "review"]);
    answer(
        &mut terminal,
        "No open pull requests of frontend.\r\n\
         Pull request or branch of frontend [none, base `main`]: ",
        "",
    );
    terminal
        .exp_string(
            "Open pull requests of backend:\r\n  \
             #12  Change 12  (feat/login, alice)\r\n  \
             #13  Change 13  (patch-1, alice, from a fork)\r\n",
        )
        .unwrap();
    let backend = "Pull request or branch of backend [none, base `main`]: ";
    answer(&mut terminal, backend, "13");
    terminal
        .exp_string("pull request #13 of `backend` comes from a fork")
        .unwrap();
    answer(&mut terminal, backend, "#12");
    let agent = "Pull request or branch of agent [none, base `main`]: ";
    answer(&mut terminal, agent, "fix/nope");
    terminal
        .exp_string("repo `agent` has no branch `fix/nope` on its remote")
        .unwrap();
    answer(&mut terminal, agent, "fix/prompt");
    let printed = finish(terminal);

    assert!(
        printed.contains("created workspace `pr-backend-12` in deeptree"),
        "{printed}"
    );
    let path = env.workspace_dir("pr-backend-12");
    assert_eq!(current_branch(&path.join("frontend")), "pr-backend-12");
    assert_eq!(current_branch(&path.join("backend")), "feat/login");
    assert_eq!(current_branch(&path.join("agent")), "fix/prompt");
}

#[test]
fn pr_review_at_a_terminal_with_nothing_chosen_creates_nothing() {
    let env = Env::with_repos(&["frontend"]);

    let mut terminal = env.terminal(&["pr", "review"]);
    answer(
        &mut terminal,
        "Pull request or branch of frontend [none, base `main`]: ",
        "",
    );
    terminal.exp_string("error: nothing to review").unwrap();
    terminal.exp_eof().unwrap();

    assert!(!env.dir.path().join("workspaces").exists());
}

#[test]
fn pr_review_lists_pull_requests_in_one_readable_line_each() {
    let env = Env::with_repos(&["backend"]);
    let long_title = "Rework the whole login flow ".repeat(5);
    for (number, title, author) in [
        (
            7,
            "Fix\u{1b}[31m the\r\ncolours",
            serde_json::json!({"login": "bob"}),
        ),
        (120, long_title.as_str(), serde_json::Value::Null),
    ] {
        let pull_request = serde_json::json!({
            "number": number,
            "title": title,
            "headRefName": "feat/x",
            "author": author,
            "isCrossRepository": false,
        });
        env.gh_file(
            "backend",
            &format!("{number}.json"),
            &pull_request.to_string(),
        );
    }

    let mut terminal = env.terminal(&["pr", "review"]);
    terminal
        .exp_string(
            "Open pull requests of backend:\r\n  \
             #120  Rework the whole login flow Rework the whole login flow Rework the wh...  \
             (feat/x)\r\n  \
             #7    Fix [31m the  colours  (feat/x, bob)\r\n",
        )
        .unwrap();
    answer(
        &mut terminal,
        "Pull request or branch of backend [none, base `main`]: ",
        "",
    );
    terminal.exp_eof().unwrap();
}

#[test]
fn pr_review_still_asks_for_a_repo_whose_pull_requests_cannot_be_listed() {
    let env = Env::with_a_pull_request();
    env.gh_file("frontend", "error", "error connecting to api.github.com\n");

    let mut terminal = env.terminal(&["pr", "review"]);
    terminal
        .exp_string("cannot list the pull requests of frontend:")
        .unwrap();
    terminal
        .exp_string("error connecting to api.github.com")
        .unwrap();
    let asked = terminal
        .exp_string("Pull request or branch of frontend [none, base `main`]: ")
        .unwrap();
    assert!(!asked.contains("No open pull requests"), "{asked}");
    terminal.send_line("").unwrap();
    answer(
        &mut terminal,
        "Pull request or branch of backend [none, base `main`]: ",
        "12",
    );
    answer(
        &mut terminal,
        "Pull request or branch of agent [none, base `main`]: ",
        "",
    );
    let printed = finish(terminal);

    assert!(
        printed.contains("created workspace `pr-backend-12` in deeptree"),
        "{printed}"
    );
}

#[test]
fn pr_review_stops_before_any_question_when_it_cannot_succeed() {
    let empty = Env::with_repos(&[]);
    let env = Env::with_a_pull_request();

    let mut terminal = empty.terminal(&["pr", "review"]);
    terminal
        .exp_string("error: project `deeptree` has no repos")
        .unwrap();
    terminal.exp_eof().unwrap();
    let mut terminal = env.terminal(&["pr", "review", "--name", "my review"]);
    terminal
        .exp_string("error: invalid branch `my review`")
        .unwrap();
    let rest = terminal.exp_eof().unwrap();

    assert!(!rest.contains("Pull request or branch"), "{rest}");
}

#[test]
fn json_is_accepted_before_and_after_any_command() {
    let env = Env::with_repos(&["frontend"]);

    for args in [["--json", "status"], ["status", "--json"]] {
        let (code, status) = env.document(&args);
        assert_eq!(code, Some(0));
        assert_eq!(status["schema"], 1);
        assert_eq!(status["project"], "deeptree");
    }
}

#[test]
fn project_commands_print_the_resulting_project_as_json() {
    let env = Env::new();
    let file = env.dir.path().join("projects/deeptree.yaml");
    let project = |base: &str| {
        serde_json::json!({
            "schema": 1,
            "project": "deeptree",
            "base": base,
            "location": "local",
            "file": file,
        })
    };

    let created = env.document(&["project", "init", "deeptree", "--base", "main", "--json"]);
    assert_eq!(created, (Some(0), project("main")));
    let changed = env.document(&["project", "set", "--base", "develop", "--json"]);
    assert_eq!(changed, (Some(0), project("develop")));
    let used = env.document(&["project", "use", "deeptree", "--json"]);
    assert_eq!(
        used,
        (
            Some(0),
            serde_json::json!({"schema": 1, "default": "deeptree"})
        )
    );
    let removed = env.document(&["project", "rm", "deeptree", "--json"]);
    assert_eq!(
        removed,
        (
            Some(0),
            serde_json::json!({"schema": 1, "removed": "deeptree", "default": null})
        )
    );
}

#[test]
fn project_set_location_json_reports_the_repos_cloned_again() {
    let env = Env::with_repos(&["frontend"]);

    let (code, moved) = env.document(&["project", "set", "--location", "vm", "--json"]);

    assert_eq!(code, Some(0));
    assert_eq!(moved["location"], env.machine_location("vm"));
    assert_eq!(
        moved["repos"],
        serde_json::json!([{"key": "frontend", "outcome": "cloned"}])
    );
}

#[test]
fn repo_commands_print_the_resulting_repo_as_json() {
    let env = Env::with_repos(&[]);
    let url = env.remote_with_branch("frontend", "develop");
    let repo = |alias: Option<&str>, base: &str| {
        serde_json::json!({
            "key": "frontend",
            "alias": alias,
            "url": url,
            "base": base,
            "location": "local",
        })
    };

    let added = env.document(&["repo", "add", &url, "--alias", "web", "--json"]);
    assert_eq!(
        added,
        (
            Some(0),
            serde_json::json!({"schema": 1, "project": "deeptree", "repo": repo(Some("web"), "main")})
        )
    );
    let changed = env.document(&[
        "repo",
        "set",
        "web",
        "--no-alias",
        "--base",
        "develop",
        "--json",
    ]);
    assert_eq!(
        changed,
        (
            Some(0),
            serde_json::json!({"schema": 1, "project": "deeptree", "repo": repo(None, "develop")})
        )
    );
    let in_workspace = env.document(&[
        "repo",
        "set",
        "frontend",
        "--run",
        "TOKEN=secret npm start",
        "--branch",
        "feat/login",
        "--json",
    ]);
    assert_eq!(
        in_workspace,
        (
            Some(0),
            serde_json::json!({
                "schema": 1,
                "project": "deeptree",
                "workspace": "feat-login",
                "repo": repo(None, "develop"),
            })
        )
    );
    let removed = env.document(&["repo", "rm", "frontend", "--json"]);
    assert_eq!(
        removed,
        (
            Some(0),
            serde_json::json!({"schema": 1, "project": "deeptree", "removed": "frontend"})
        )
    );
}

#[test]
fn fetch_json_reports_every_repo_and_exits_6_when_one_fails() {
    let env = Env::with_repos(&["frontend", "backend"]);
    fs::remove_dir_all(env.dir.path().join("cache/deeptree/backend.git")).unwrap();

    let fetched = env.document(&["fetch", "--json"]);
    assert_eq!(
        fetched,
        (
            Some(0),
            serde_json::json!({
                "schema": 1,
                "project": "deeptree",
                "repos": [
                    {"key": "frontend", "outcome": "fetched"},
                    {"key": "backend", "outcome": "cloned"},
                ],
            })
        )
    );

    fs::remove_dir_all(env.remote_dir("frontend")).unwrap();
    let out = env.octok(&["fetch", "--json"]).output().unwrap();
    let failed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(6));
    assert_eq!(failed["repos"][0]["key"], "frontend");
    assert_eq!(failed["repos"][0]["error"]["code"], "command_failed");
    let message = failed["repos"][0]["error"]["message"].as_str().unwrap();
    assert!(message.contains("git fetch"), "{message}");
    assert_eq!(
        failed["repos"][1],
        serde_json::json!({"key": "backend", "outcome": "fetched"})
    );
    // What git itself says is for people: the document already has the error.
    assert!(!String::from_utf8_lossy(&out.stderr).contains("error: frontend"));
}

#[test]
fn branch_and_checkout_json_describe_the_workspace_and_each_checkout() {
    let env = Env::with_repos(&["frontend", "backend"]);
    commit_on_new_branch(&env.remote_dir("backend"), "feat/shared", COMMIT_DATE);
    let workspace = |name: &str, frontend: &str, backend: &str| {
        let path = env.workspace_dir(name);
        serde_json::json!({
            "schema": 1,
            "project": "deeptree",
            "workspace": name,
            "path": path,
            "repos": [
                {"key": "frontend", "path": path.join("frontend"), "outcome": frontend},
                {"key": "backend", "path": path.join("backend"), "outcome": backend},
            ],
        })
    };

    let out = env
        .octok(&["branch", "feat/login", "--json"])
        .output()
        .unwrap();
    let created: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(created, workspace("feat-login", "created", "created"));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("created workspace"));

    let kept = env.document(&["checkout", "feat/login", "--json"]);
    assert_eq!(kept, (Some(0), workspace("feat-login", "kept", "kept")));

    fs::remove_dir_all(env.workspace_dir("feat-login").join("frontend")).unwrap();
    let restored = env.document(&["checkout", "feat/login", "--json"]);
    assert_eq!(
        restored,
        (Some(0), workspace("feat-login", "restored", "kept"))
    );

    let tracked = env.document(&["checkout", "feat/shared", "--json"]);
    assert_eq!(
        tracked,
        (Some(0), workspace("feat-shared", "created", "tracked"))
    );
}

#[test]
fn checkout_json_gives_the_path_of_a_checkout_on_another_machine() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["repo", "add", &env.remote("backend"), "--location", "vm"])
        .assert()
        .success();

    let (code, created) = env.document(&["checkout", "-b", "feat/login", "--json"]);

    assert_eq!(code, Some(0));
    let here = env.workspace_dir("feat-login");
    let there = env.machine_root().join("workspaces/deeptree/feat-login");
    assert_eq!(created["path"], serde_json::json!(here));
    assert_eq!(
        created["repos"][0]["path"],
        serde_json::json!(here.join("frontend"))
    );
    assert_eq!(
        created["repos"][1]["path"],
        format!("vm:{}", there.join("backend").display())
    );
}

#[test]
fn pr_review_json_describes_the_workspace() {
    let env = Env::with_a_pull_request();

    let (code, review) = env.document(&["pr", "review", "--pr", "backend=12", "--json"]);

    assert_eq!(code, Some(0));
    assert_eq!(review["workspace"], "pr-backend-12");
    assert_eq!(
        review["path"],
        serde_json::json!(env.workspace_dir("pr-backend-12"))
    );
    let outcomes: Vec<&str> = review["repos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|repo| repo["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(outcomes, ["created", "tracked", "created"]);
}

#[test]
fn pull_json_reports_the_outcome_of_each_repo() {
    let env = Env::with_repos(&["frontend", "backend", "agent"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    for name in ["frontend", "agent"] {
        git(
            &env.remote_dir(name),
            &["checkout", "--quiet", "-b", "feat/login"],
        );
    }
    commit(&env.remote_dir("frontend"), "pushed by a teammate");
    fs::remove_dir_all(env.remote_dir("backend")).unwrap();

    let out = env
        .octok(&["pull", "feat/login", "--json"])
        .output()
        .unwrap();

    let pulled: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(6));
    assert_eq!(pulled["schema"], 1);
    assert_eq!(pulled["project"], "deeptree");
    assert_eq!(pulled["workspace"], "feat-login");
    assert_eq!(
        pulled["repos"][0],
        serde_json::json!({"key": "frontend", "outcome": "updated"})
    );
    assert_eq!(pulled["repos"][1]["error"]["code"], "command_failed");
    assert_eq!(
        pulled["repos"][2],
        serde_json::json!({"key": "agent", "outcome": "up_to_date"})
    );
}

#[test]
fn rm_json_says_what_was_removed_or_why_it_was_refused() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    fs::write(
        env.workspace_dir("feat-login").join("frontend/draft.txt"),
        "wip",
    )
    .unwrap();

    let out = env.octok(&["rm", "feat/login", "--json"]).output().unwrap();
    let refused: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert_eq!(out.stderr, b"");
    assert_eq!(refused["schema"], 1);
    let error = &refused["error"];
    assert_eq!(error["code"], "workspace_has_work");
    assert_eq!(error["workspace"], "feat-login");
    assert_eq!(
        error["reasons"],
        serde_json::json!(["frontend: uncommitted changes"])
    );
    let message = error["message"].as_str().unwrap();
    assert!(
        message.starts_with("workspace `feat-login` has work"),
        "{message}"
    );

    let removed = env.document(&["rm", "--force", "feat/login", "--json"]);
    assert_eq!(
        removed,
        (
            Some(0),
            serde_json::json!({"schema": 1, "project": "deeptree", "removed": "feat-login"})
        )
    );
    let (code, missing) = env.document(&["rm", "feat/login", "--json"]);
    assert_eq!(code, Some(3));
    assert_eq!(missing["error"]["code"], "workspace_not_found");
    // The name of the workspace, as `octok pull` gives it, not the branch as typed.
    assert_eq!(missing["error"]["workspace"], "feat-login");
    let (code, missing) = env.document(&["pull", "feat/login", "--json"]);
    assert_eq!(code, Some(3));
    assert_eq!(missing["error"]["workspace"], "feat-login");
}

#[test]
fn a_failure_exits_with_the_code_of_its_class() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    let code_of = |args: &[&str]| env.document(args).1["error"]["code"].clone();

    // 2: the command is not written the way octok takes it.
    env.octok(&["repo", "add", "not-a-url"]).assert().code(2);
    assert_eq!(
        code_of(&["repo", "add", "not-a-url", "--json"]),
        "invalid_url"
    );
    env.octok(&["pull"])
        .current_dir(env.dir.path())
        .assert()
        .code(2);
    // 3: what it names is not there.
    env.octok(&["repo", "rm", "nope"]).assert().code(3);
    assert_eq!(code_of(&["repo", "rm", "nope", "--json"]), "repo_not_found");
    env.octok(&["-p", "nope", "status"]).assert().code(3);
    // 5: it conflicts with what exists.
    env.octok(&["project", "init", "deeptree"]).assert().code(5);
    env.octok(&["branch", "feat/login"]).assert().code(5);
    assert_eq!(
        code_of(&["branch", "feat/login", "--json"]),
        "workspace_exists"
    );
    env.octok(&["repo", "rm", "frontend"]).assert().code(5);
    // 7: a machine or a program cannot be reached.
    env.octok(&["project", "set", "--location", "unreachable"])
        .assert()
        .code(7);
    let unreachable = ["project", "set", "--location", "unreachable", "--json"];
    assert_eq!(env.document(&unreachable).1["error"]["host"], "unreachable");
    // 1: anything else.
    env.octok(&["repo", "add", &env.remote_url("missing")])
        .assert()
        .code(1);
}

#[test]
fn a_failure_in_text_stays_on_stderr() {
    let env = Env::with_repos(&[]);

    env.octok(&["repo", "rm", "nope"])
        .assert()
        .code(3)
        .stdout("")
        .stderr("error: repo `nope` is not registered\n");
}

#[test]
fn a_command_octok_cannot_read_is_a_json_failure_too() {
    let env = Env::with_repos(&["frontend"]);

    for args in [
        vec!["--json", "nope"],
        vec!["repo", "set", "frontend", "--json"],
        vec!["status", "--json", "--nope"],
        vec!["status", "--json=true"],
    ] {
        let out = env.octok(&args).output().unwrap();
        let failure: serde_json::Value =
            serde_json::from_slice(&out.stdout).unwrap_or_else(|error| panic!("{args:?}: {error}"));
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert_eq!(out.stderr, b"", "{args:?}");
        assert_eq!(failure["schema"], 1);
        assert_eq!(failure["error"]["code"], "usage");
        let message = failure["error"]["message"].as_str().unwrap();
        assert!(message.contains("Usage: octok"), "{message}");
        assert!(!message.starts_with("error: "), "{message}");
    }
    env.octok(&["nope"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::starts_with("error: "));
    env.octok(&["--help", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("octotask: "));
}

#[test]
fn json_at_a_terminal_asks_nothing() {
    let env = Env::new();

    let printed = finish(env.terminal(&["project", "init", "deeptree", "--json"]));
    assert_eq!(last_document(&printed)["base"], "staging");

    let url = env.remote_with_branch("frontend", "staging");
    let printed = finish(env.terminal(&["repo", "add", &url, "--json"]));
    assert_eq!(last_document(&printed)["repo"]["key"], "frontend");
    assert_eq!(
        last_document(&printed)["repo"]["alias"],
        serde_json::Value::Null
    );

    let mut terminal = env.terminal(&["pr", "review", "--json"]);
    let printed = terminal.exp_eof().unwrap();
    assert_eq!(
        last_document(&printed)["error"]["code"],
        "nothing_to_review"
    );
}

#[test]
fn json_at_a_terminal_refuses_a_base_missing_on_the_remote_without_asking() {
    let env = Env::with_repos(&[]);
    let url = env.remote("frontend");

    let mut terminal = env.terminal(&["repo", "add", &url, "--base", "nope", "--json"]);
    let printed = terminal.exp_eof().unwrap();

    let failure = last_document(&printed);
    assert_eq!(failure["error"]["code"], "base_not_found");
    assert_eq!(failure["error"]["repo"], "frontend");
    assert_eq!(failure["error"]["base"], "nope");
}

#[test]
fn json_at_a_terminal_keeps_ssh_from_asking() {
    let env = Env::with_repos(&[]);

    finish(env.terminal(&["project", "set", "--location", "vm", "--json"]));

    let log = fs::read_to_string(env.machine_file("ssh.log")).unwrap();
    let connections: Vec<&str> = log.lines().filter(|line| line.contains("-- vm ")).collect();
    assert!(!connections.is_empty(), "{log}");
    for connection in connections {
        assert!(connection.contains("BatchMode=yes"), "{connection}");
    }
}

#[test]
fn set_ssh_passphrase_is_refused_with_json() {
    let env = Env::with_repos(&[]);

    let (code, refused) = env.document(&["set-ssh-passphrase", "vm", "--json"]);

    assert_eq!(code, Some(2));
    assert_eq!(refused["error"]["code"], "needs_terminal");
    assert!(!env.machine_file("ssh-add.log").exists());
    assert!(!env.machine_file("ssh.log").exists());
}

#[test]
fn dev_save_json_is_the_document_of_dev_show() {
    let env = env_with_app(None);
    let login = env.workspace_dir("feat-login").join("backend");
    fs::write(
        login.join("app.py"),
        APP_PY.replace("port=8000", "port=9005"),
    )
    .unwrap();
    fs::write(login.join(".env"), "TOKEN=secret\n").unwrap();

    let saved = env.document(&[
        "dev",
        "save",
        "--file",
        ".env",
        "--branch",
        "feat/login",
        "--json",
    ]);

    assert_eq!(saved.0, Some(0));
    assert_eq!(saved, env.document(&["dev", "show", "--json"]));
    assert_eq!(saved.1["repos"][0]["new"], serde_json::json!([".env"]));
    assert!(!saved.1.to_string().contains("secret"), "{}", saved.1);
}

#[test]
fn up_json_prints_one_event_per_line() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&["repo", "set", "frontend", "--run", "echo hello; exit 3"])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();

    let out = env
        .octok(&["up", "--branch", "feat/login", "--json"])
        .output()
        .unwrap();

    let events: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{error}: {line}")))
        .collect();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stderr, b"");
    assert_eq!(
        events,
        [
            serde_json::json!({
                "schema": 1,
                "event": "started",
                "project": "deeptree",
                "workspace": "feat-login",
                "repos": ["frontend"],
            }),
            serde_json::json!({
                "schema": 1,
                "event": "output",
                "repo": "frontend",
                "line": "$ echo hello; exit 3",
            }),
            serde_json::json!({
                "schema": 1,
                "event": "output",
                "repo": "frontend",
                "line": "hello",
            }),
            serde_json::json!({
                "schema": 1,
                "event": "exited",
                "repo": "frontend",
                "code": 3,
                "signal": null,
            }),
        ]
    );
}

#[test]
fn up_json_reports_a_repo_stopped_on_ctrl_c_with_the_exit_code_of_its_signal() {
    let sleeping = Sleeping::here();
    let mut up = Up::start_with(&sleeping.env, &["up", "--branch", "feat/login", "--json"]);
    up.wait_for_line(r#"{"schema":1,"event":"output","repo":"frontend","line":"ready"}"#);

    up.signal("-INT");
    let mut rest = String::new();
    std::io::Read::read_to_string(&mut up.stdout, &mut rest).unwrap();
    let (code, _stderr) = up.finish();

    assert_eq!(code, Some(130));
    let exited: serde_json::Value = serde_json::from_str(rest.lines().last().unwrap()).unwrap();
    assert_eq!(
        exited,
        serde_json::json!({
            "schema": 1,
            "event": "exited",
            "repo": "frontend",
            // 128 + SIGTERM, as a shell reports a command that a signal ended.
            "code": 143,
            "signal": null,
        })
    );
    sleeping.assert_stopped();
}

#[test]
fn up_json_fails_with_a_document_when_nothing_can_start() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();

    let (code, failure) = env.document(&["up", "--branch", "feat/login", "--json"]);

    assert_eq!(code, Some(3));
    assert_eq!(failure["error"]["code"], "nothing_to_run");
    assert_eq!(failure["error"]["workspace"], "feat-login");
}

#[test]
fn status_json_says_why_a_checkout_cannot_be_read_as_an_error_document() {
    let env = Env::with_repos(&["frontend", "backend"]);
    env.octok(&["branch", "feat/login"]).assert().success();
    fs::remove_dir_all(env.dir.path().join("cache/deeptree/backend.git/worktrees")).unwrap();

    let (code, status) = env.document(&["status", "--json"]);

    assert_eq!(code, Some(0));
    let checkouts = &status["workspaces"][0]["checkouts"];
    assert_eq!(checkouts[0]["error"], serde_json::Value::Null);
    assert_eq!(checkouts[1]["commit"], serde_json::Value::Null);
    assert_eq!(checkouts[1]["error"]["code"], "command_failed");
    let message = checkouts[1]["error"]["message"].as_str().unwrap();
    assert!(message.contains("fatal: "), "{message}");
}

#[test]
fn a_reader_that_goes_away_ends_octok_quietly() {
    let env = Env::with_repos(&["frontend"]);
    let run = "echo ready; sleep 1; echo more; echo more; sleep 300";
    env.octok(&["repo", "set", "frontend", "--run", run])
        .assert()
        .success();
    env.octok(&["branch", "feat/login"]).assert().success();
    let mut up = Up::start_with(&env, &["up", "--branch", "feat/login", "--json"]);
    up.wait_for_line(r#"{"schema":1,"event":"output","repo":"frontend","line":"ready"}"#);

    drop(up.stdout);
    let output = up.octok.wait_with_output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(
        std::os::unix::process::ExitStatusExt::signal(&output.status),
        Some(13),
        "ended by SIGPIPE, as any command whose reader went away: {stderr}"
    );
}
