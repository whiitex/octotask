use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

struct Env {
    dir: TempDir,
}

impl Env {
    fn new() -> Self {
        Self {
            dir: TempDir::new().unwrap(),
        }
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
}

#[test]
fn registered_repos_are_persisted_in_project_file() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree", "--base", "develop"])
        .assert()
        .success();
    env.octok(&["repo", "add", "git@github.com:org/frontend.git"])
        .assert()
        .success();
    env.octok(&[
        "repo",
        "add",
        "git@github.com:org/financial-agent.git",
        "--name",
        "agent",
        "--base",
        "main",
    ])
    .assert()
    .success();

    let raw = fs::read_to_string(env.dir.path().join("projects/deeptree.yaml")).unwrap();
    assert!(raw.contains("git@github.com:org/frontend.git"), "{raw}");

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["project"], "deeptree");
    assert_eq!(
        ls["repos"],
        serde_json::json!([
            {"key": "frontend", "url": "git@github.com:org/frontend.git", "base": "develop"},
            {"key": "agent", "url": "git@github.com:org/financial-agent.git", "base": "main"},
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
    env.octok(&["repo", "add", "git@github.com:org/frontend.git"])
        .assert()
        .success();
    env.octok(&["-p", "side", "repo", "add", "git@github.com:org/tool.git"])
        .assert()
        .success();

    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"][0]["key"],
        "frontend"
    );
    assert_eq!(
        env.json(&["repo", "ls", "-p", "side", "--json"])["repos"][0]["key"],
        "tool"
    );

    env.octok(&["project", "use", "side"]).assert().success();
    assert_eq!(env.json(&["repo", "ls", "--json"])["project"], "side");
}

#[test]
fn project_ls_reports_default() {
    let env = Env::new();
    env.octok(&["project", "init", "zeta"]).assert().success();
    env.octok(&["project", "init", "alpha"]).assert().success();
    let ls = env.json(&["project", "ls", "--json"]);
    assert_eq!(ls["projects"], serde_json::json!(["alpha", "zeta"]));
    assert_eq!(ls["default"], "zeta");
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
        .stdout("  alpha\n* zeta\n");
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

    env.octok(&["repo", "add", "git@github.com:org/frontend.git"])
        .assert()
        .success();
    env.octok(&[
        "repo",
        "add",
        "git@github.com:org/api.git",
        "--base",
        "main",
    ])
    .assert()
    .success();
    env.octok(&["repo", "ls"]).assert().success().stdout(
        "frontend  develop  git@github.com:org/frontend.git\n\
         api       main     git@github.com:org/api.git\n",
    );
}

#[test]
fn repo_rm_removes_from_file() {
    let env = Env::new();
    env.octok(&["project", "init", "deeptree"])
        .assert()
        .success();
    env.octok(&["repo", "add", "git@github.com:org/frontend.git"])
        .assert()
        .success();
    env.octok(&["repo", "rm", "frontend"]).assert().success();
    assert_eq!(
        env.json(&["repo", "ls", "--json"])["repos"],
        serde_json::json!([])
    );
    env.octok(&["repo", "rm", "frontend"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("frontend"));
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
    env.octok(&["repo", "add", "git@github.com:org/frontend.git"])
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
    env.octok(&["repo", "add", "git@github.com:org/frontend.git"])
        .assert()
        .success();
    env.octok(&["repo", "add", "git@github.com:other/frontend.git"])
        .assert()
        .code(1);

    let ls = env.json(&["repo", "ls", "--json"]);
    assert_eq!(ls["repos"].as_array().unwrap().len(), 1);
    assert_eq!(ls["repos"][0]["url"], "git@github.com:org/frontend.git");
}
