use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

struct Env {
    dir: TempDir,
    remotes: TempDir,
}

impl Env {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
            remotes: TempDir::new().unwrap(),
        }
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
        env.octok(&["project", "init", "deeptree"])
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
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("octok"));
        cmd.env("OCTOTASK_HOME", self.dir.path()).args(args);
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

    fn read(&self, relative_path: &str) -> String {
        fs::read_to_string(self.dir.path().join(relative_path)).unwrap()
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
            {"key": "frontend", "alias": null, "url": env.remote_url("frontend"), "base": "develop"},
            {"key": "agent", "alias": null, "url": env.remote_url("financial-agent"), "base": "main"},
        ])
    );
}

#[test]
fn repo_commands_target_default_project_unless_overridden() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["project", "init", "side"]).assert().success();
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
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["project", "init", "side"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "`side` is now the default project\n",
        ));
}

#[test]
fn project_rm_of_another_project_keeps_the_default() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["project", "init", "side"]).assert().success();

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
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["project", "init", "side"]).assert().success();

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
        .code(1)
        .stderr(predicate::str::contains("default project"));
    env.octok(&["project", "rm", "side"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("project `side` not found"));
}

#[test]
fn project_ls_reports_default() {
    let env = Env::new();
    env.octok(&["project", "init", "zeta"]).assert().success();
    env.octok(&["project", "init", "alpha"]).assert().success();
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

    env.octok(&["project", "init", "zeta"]).assert().success();
    env.octok(&["project", "init", "alpha"]).assert().success();
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
    env.octok(&["project", "init", "deeptree"])
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
            {"key": "backend", "alias": null, "url": env.remote_url("backend"), "base": "main"},
        ])
    );
    let after_rm = env.read("projects/deeptree.yaml");
    assert!(!after_rm.contains("frontend"), "{after_rm}");
    assert!(after_rm.contains(&env.remote_url("backend")), "{after_rm}");

    env.octok(&["repo", "rm", "frontend"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("frontend"));
    assert_eq!(env.read("projects/deeptree.yaml"), after_rm);
}

#[test]
fn concurrent_repo_adds_are_all_saved() {
    const REPOS: usize = 24;
    let env = Env::new();
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();

    let children: Vec<_> = (0..REPOS)
        .map(|i| {
            env.octok(&["repo", "add", &env.remote(&format!("repo{i}"))])
                .stdout(Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    for mut child in children {
        assert!(child.wait().unwrap().success());
    }

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["repos"].as_array().unwrap().len(), REPOS);
}

#[test]
fn repo_add_with_name_accepts_url_whose_last_segment_is_not_a_valid_key() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote(".github"), "--name", "dotgithub"])
        .assert()
        .success();

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([
            {"key": "dotgithub", "alias": null, "url": env.remote_url(".github"), "base": "main"},
        ])
    );
}

#[test]
fn repo_add_without_name_asks_for_one_when_no_key_can_be_derived() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    let before = env.read("projects/deeptree.yaml");

    env.octok(&["repo", "add", &env.remote(".github")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--name"))
        .stderr(predicate::str::contains("invalid git url").not());

    assert_eq!(env.read("projects/deeptree.yaml"), before);
}

#[test]
fn newer_global_config_is_rejected_and_left_untouched() {
    let env = Env::new();
    let config = env.dir.path().join("config.yaml");
    fs::write(&config, "version: 99\n").unwrap();

    env.octok(&["project", "init", "deeptree"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unsupported config version 99 (expected 1)",
        ));

    assert_eq!(fs::read_to_string(&config).unwrap(), "version: 99\n");
    assert!(!env.dir.path().join("projects/deeptree.yaml").exists());
}

#[test]
fn errors_exit_1_with_message_and_change_nothing() {
    let env = Env::new();
    env.octok(&["repo", "add", &env.remote("frontend")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("default project"));

    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["repo", "add", "not-a-url"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not-a-url"));
    env.octok(&["repo", "add", &env.remote("frontend")])
        .assert()
        .success();
    env.octok(&["repo", "add", &env.remote("other/frontend")])
        .assert()
        .code(1);

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
        .code(1)
        .stderr(predicate::str::contains("feat-login"));
}

#[test]
fn fetch_reports_every_repo_and_exits_1_when_one_fails() {
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
        .code(1)
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
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("octok checkout -b feat/login"));

    assert_eq!(env.stdout(&["checkout", "-b", "feat/login"]), path);
    assert_eq!(env.stdout(&["checkout", "feat/login"]), path);
}

#[test]
fn branch_that_already_exists_is_refused_with_a_hint() {
    let env = Env::with_repos(&["frontend", "backend"]);
    git(&env.remote_dir("backend"), &["branch", "feat/login"]);

    env.octok(&["branch", "feat/login"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(
            "error: branch `feat/login` already exists on the remote of: backend\n\
             choose another name, or delete the existing branch first\n",
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
    env.octok(&["project", "init", "side"]).assert().success();
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
        .code(1)
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
         workspaces:\n  feat-login  {}\n    frontend  feat/login  {line}\n",
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
        .code(1)
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
    .code(1)
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
        .code(1)
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
             \x20 new branch: backend\n",
        );
    env.octok(&["branch", "feat/login"])
        .assert()
        .code(1)
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

    env.octok(&["rm", "feat/login"]).assert().code(1).stderr(
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
        .code(1)
        .stderr(predicate::str::contains("no workspace for `feat/login`"));
    env.octok(&["branch", "feat/login"]).assert().success();
}

#[test]
fn project_rm_is_refused_with_workspaces_and_then_deletes_the_caches() {
    let env = Env::with_repos(&["frontend"]);
    env.octok(&["branch", "feat/login"]).assert().success();

    env.octok(&["project", "rm", "deeptree"])
        .assert()
        .code(1)
        .stderr(
            "error: project `deeptree` still has workspaces: feat-login\n\
             remove them first with `octok rm <branch>`\n",
        );
    assert_eq!(env.project_dir_entries(), ["deeptree.yaml"]);

    env.octok(&["rm", "feat/login"]).assert().success();
    env.octok(&["project", "rm", "deeptree"]).assert().success();
    assert!(!env.dir.path().join("cache/deeptree").exists());

    // A new project with the same name starts empty.
    env.octok(&["project", "init", "deeptree"])
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
            "    backend   no worktree: add it with `octok branch feat/login`\n",
        ));
}
