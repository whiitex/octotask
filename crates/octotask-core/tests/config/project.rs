use std::collections::HashMap;

use octotask_core::{Error, Location, Project, repo_key_from_url};

#[test]
fn repo_key_is_derived_from_common_url_forms() {
    let cases = [
        ("git@github.com:org/frontend.git", "frontend"),
        ("https://github.com/org/backend.git", "backend"),
        ("https://github.com/org/backend", "backend"),
        ("https://github.com/org/backend.git/", "backend"),
        (
            "ssh://git@host:2222/team/financial-agent.git",
            "financial-agent",
        ),
        ("git@host:agent", "agent"),
        ("/srv/git/infra.git", "infra"),
        ("file:///srv/git/infra", "infra"),
    ];
    for (url, want) in cases {
        assert_eq!(repo_key_from_url(url).unwrap(), want, "url: {url}");
    }
}

#[test]
fn malformed_urls_are_rejected() {
    for url in [
        "",
        "   ",
        "frontend",
        "relative/path",
        "https://",
        "git@github.com:",
        "has space/repo.git",
    ] {
        assert!(
            matches!(repo_key_from_url(url), Err(Error::InvalidUrl(_))),
            "should reject {url:?}"
        );
    }
}

#[test]
fn malformed_urls_are_rejected_even_with_an_explicit_key() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    for url in [
        "",
        "frontend",
        "relative/path",
        "https://",
        "https://github.com/",
        "git@github.com:",
        "/",
        "has space/repo.git",
    ] {
        assert!(
            matches!(
                p.add_repo(url, Some("key"), None),
                Err(Error::InvalidUrl(_))
            ),
            "should reject {url:?}"
        );
    }
    assert!(p.repos.is_empty());
}

#[test]
fn explicit_key_allows_urls_without_a_derivable_key() {
    let long_name = "x".repeat(65);
    let long_url = format!("https://github.com/org/{long_name}.git");
    for url in [
        "git@github.com:org/.github.git",
        "https://github.com/org/c++.git",
        "/srv/git/project/.git",
        long_url.as_str(),
    ] {
        let mut p = Project::new("deeptree", "develop").unwrap();
        assert_eq!(p.add_repo(url, Some("key"), None).unwrap(), "key");
        assert_eq!(p.repos["key"].url, url);
    }
}

#[test]
fn missing_key_for_underivable_url_asks_for_explicit_one() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    let err = p
        .add_repo("git@github.com:org/.github.git", None, None)
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "cannot derive a repo key from `git@github.com:org/.github.git`: choose one with `--name <key>`"
    );
    assert!(p.repos.is_empty());
}

#[test]
fn add_repo_uses_derived_key_and_keeps_insertion_order() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    assert_eq!(
        p.add_repo("git@github.com:org/frontend.git", None, None)
            .unwrap(),
        "frontend"
    );
    assert_eq!(
        p.add_repo("git@github.com:org/backend.git", None, None)
            .unwrap(),
        "backend"
    );
    assert_eq!(
        p.add_repo("git@github.com:org/agent.git", None, None)
            .unwrap(),
        "agent"
    );

    let keys: Vec<&str> = p.repos.keys().map(String::as_str).collect();
    assert_eq!(keys, ["frontend", "backend", "agent"]);
    assert_eq!(p.repos["backend"].url, "git@github.com:org/backend.git");
}

#[test]
fn add_repo_accepts_explicit_key() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    let key = p
        .add_repo(
            "git@github.com:org/financial-agent.git",
            Some("agent"),
            None,
        )
        .unwrap();
    assert_eq!(key, "agent");
    assert!(p.repos.contains_key("agent"));
}

#[test]
fn duplicate_key_is_rejected_and_project_unchanged() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    p.add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    let err = p
        .add_repo("git@gitlab.com:other/frontend.git", None, None)
        .unwrap_err();
    assert!(
        matches!(&err, Error::DuplicateRepo { key, url }
            if key == "frontend" && url == "git@github.com:org/frontend.git"),
        "{err:?}"
    );
    assert!(err.to_string().contains("--name <key>"), "{err}");
    assert_eq!(p.repos["frontend"].url, "git@github.com:org/frontend.git");
}

#[test]
fn invalid_explicit_key_is_rejected() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    for key in ["", "-x", "a b", "a/b", ".hidden"] {
        let err = p
            .add_repo("git@github.com:org/frontend.git", Some(key), None)
            .unwrap_err();
        assert!(
            matches!(
                err,
                Error::InvalidName {
                    kind: "repo key",
                    ..
                }
            ),
            "key {key:?}"
        );
    }
    assert!(p.repos.is_empty());
}

#[test]
fn invalid_project_name_or_base_is_rejected() {
    assert!(matches!(
        Project::new("my project", "main"),
        Err(Error::InvalidName {
            kind: "project name",
            ..
        })
    ));
    assert!(matches!(
        Project::new("ok", ""),
        Err(Error::InvalidName { kind: "branch", .. })
    ));
    assert!(matches!(
        Project::new("ok", "-main"),
        Err(Error::InvalidName { kind: "branch", .. })
    ));
}

#[test]
fn a_branch_name_that_git_refuses_is_rejected() {
    let refused = [
        "HEAD",
        "@",
        "a@{b",
        "/feat",
        "feat//login",
        ".hidden",
        "feat/.hidden",
        "feat.",
        "feat.lock/login",
    ];
    for name in refused {
        assert!(
            matches!(
                Project::new("ok", name),
                Err(Error::InvalidName { kind: "branch", .. })
            ),
            "{name}"
        );
    }
    for name in ["feat/login", "feat/a/b", "release-1.2", "fix_x", "user@fix"] {
        assert!(Project::new("ok", name).is_ok(), "{name}");
    }
}

#[test]
fn effective_base_prefers_repo_override() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    p.add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    p.add_repo("git@github.com:org/agent.git", None, Some("main"))
        .unwrap();
    assert_eq!(p.effective_base(&p.repos["frontend"]), "develop");
    assert_eq!(p.effective_base(&p.repos["agent"]), "main");
}

#[test]
fn remove_repo_returns_it_and_errors_when_missing() {
    let mut p = Project::new("deeptree", "develop").unwrap();
    p.add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    let removed = p.remove_repo("frontend").unwrap();
    assert_eq!(removed.url, "git@github.com:org/frontend.git");
    assert!(p.repos.is_empty());
    assert!(matches!(p.remove_repo("frontend"), Err(Error::RepoNotFound(k)) if k == "frontend"));
}

fn project_with_frontend_alias() -> Project {
    let mut p = Project::new("deeptree", "main").unwrap();
    p.add_repo("git@github.com:org/DeepTreeDashboard.git", None, None)
        .unwrap();
    p.set_alias("DeepTreeDashboard", "frontend").unwrap();
    p
}

#[test]
fn repo_is_found_by_key_or_alias() {
    let p = project_with_frontend_alias();

    assert_eq!(
        p.repos["DeepTreeDashboard"].alias.as_deref(),
        Some("frontend")
    );
    assert_eq!(p.resolve_repo("frontend").unwrap(), "DeepTreeDashboard");
    assert_eq!(
        p.resolve_repo("DeepTreeDashboard").unwrap(),
        "DeepTreeDashboard"
    );
    assert!(
        matches!(p.resolve_repo("backend"), Err(Error::RepoNotFound(name)) if name == "backend")
    );
}

#[test]
fn alias_must_differ_from_every_key_and_alias() {
    let mut p = project_with_frontend_alias();
    p.add_repo("git@github.com:org/DeepTreeBackend.git", None, None)
        .unwrap();

    for (alias, owner) in [
        ("frontend", "DeepTreeDashboard"),
        ("DeepTreeDashboard", "DeepTreeDashboard"),
        ("DeepTreeBackend", "DeepTreeBackend"),
    ] {
        let err = p.set_alias("DeepTreeBackend", alias).unwrap_err();
        assert!(
            matches!(&err, Error::DuplicateAlias { alias: a, repo } if a == alias && repo == owner),
            "{alias}: {err:?}"
        );
    }
    assert_eq!(p.repos["DeepTreeBackend"].alias, None);
    assert!(matches!(
        p.set_alias("DeepTreeBackend", "back end"),
        Err(Error::InvalidName { kind: "alias", .. })
    ));
    assert!(matches!(
        p.set_alias("missing", "x"),
        Err(Error::RepoNotFound(_))
    ));
}

#[test]
fn repo_key_equal_to_an_existing_alias_is_rejected() {
    let mut p = project_with_frontend_alias();

    let err = p
        .add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap_err();

    assert!(
        matches!(&err, Error::DuplicateAlias { alias, repo }
            if alias == "frontend" && repo == "DeepTreeDashboard"),
        "{err:?}"
    );
    assert_eq!(p.repos.len(), 1);
}

#[test]
fn alias_base_and_url_of_a_repo_can_be_changed() {
    let mut p = project_with_frontend_alias();
    let key = "DeepTreeDashboard";

    p.set_alias(key, "web").unwrap();
    p.set_repo_base(key, Some("develop")).unwrap();
    p.set_repo_url(key, " git@github.com:fork/DeepTreeDashboard.git ")
        .unwrap();
    let repo = &p.repos[key];
    assert_eq!(repo.alias.as_deref(), Some("web"));
    assert_eq!(repo.base.as_deref(), Some("develop"));
    assert_eq!(repo.url, "git@github.com:fork/DeepTreeDashboard.git");

    p.clear_alias(key).unwrap();
    p.set_repo_base(key, None).unwrap();
    assert_eq!(p.repos[key].alias, None);
    assert_eq!(p.effective_base(&p.repos[key]), "main");
}

#[test]
fn invalid_repo_changes_are_rejected_and_change_nothing() {
    let mut p = project_with_frontend_alias();
    let before = p.clone();
    let key = "DeepTreeDashboard";

    assert!(matches!(
        p.set_repo_base(key, Some("bad branch")),
        Err(Error::InvalidName { kind: "branch", .. })
    ));
    assert!(matches!(
        p.set_repo_url(key, "not a url"),
        Err(Error::InvalidUrl(_))
    ));
    assert!(matches!(
        p.set_base("bad branch"),
        Err(Error::InvalidName { kind: "branch", .. })
    ));
    for result in [
        p.clear_alias("missing"),
        p.set_repo_base("missing", None),
        p.set_repo_url("missing", "git@github.com:org/x.git"),
    ] {
        assert!(matches!(result, Err(Error::RepoNotFound(_))));
    }
    assert_eq!(p, before);
}

#[test]
fn project_base_can_be_changed() {
    let mut p = project_with_frontend_alias();

    p.set_base("develop").unwrap();

    assert_eq!(p.effective_base(&p.repos["DeepTreeDashboard"]), "develop");
}

fn remote_location(host: &str) -> Location {
    format!("{host}:/home/ubuntu/.octotask").parse().unwrap()
}

#[test]
fn location_falls_back_from_workspace_to_repo_to_project_to_local() {
    let mut project = Project::new("deeptree", "main").unwrap();
    project
        .add_repo("git@github.com:org/frontend.git", None, None)
        .unwrap();
    project
        .add_repo("git@github.com:org/backend.git", None, None)
        .unwrap();
    let location = |project: &Project, key: &str, workspace: Option<&str>| {
        project
            .effective_location(&project.repos[key], workspace)
            .clone()
    };
    assert_eq!(location(&project, "backend", None), Location::Local);

    project.set_location(Some(remote_location("project-vm")));
    assert_eq!(
        location(&project, "backend", None),
        remote_location("project-vm")
    );

    project
        .set_repo_location("backend", Some(remote_location("repo-vm")))
        .unwrap();
    project
        .set_repo_location("frontend", Some(Location::Local))
        .unwrap();
    assert_eq!(
        location(&project, "backend", None),
        remote_location("repo-vm")
    );
    assert_eq!(location(&project, "frontend", None), Location::Local);

    project
        .set_workspace_location("feat-login", Some(remote_location("workspace-vm")))
        .unwrap();
    for key in ["frontend", "backend"] {
        assert_eq!(
            location(&project, key, Some("feat-login")),
            remote_location("workspace-vm")
        );
    }
    assert_eq!(
        location(&project, "backend", Some("feat-other")),
        remote_location("repo-vm")
    );

    project.set_workspace_location("feat-login", None).unwrap();
    assert_eq!(
        location(&project, "backend", Some("feat-login")),
        remote_location("repo-vm")
    );
    project.set_repo_location("backend", None).unwrap();
    assert_eq!(
        location(&project, "backend", None),
        remote_location("project-vm")
    );
}

#[test]
fn locations_lists_each_machine_of_the_project_once() {
    let mut project = Project::new("deeptree", "main").unwrap();
    assert_eq!(project.locations(), [Location::Local]);

    for name in ["frontend", "backend", "agent"] {
        project
            .add_repo(&format!("git@github.com:org/{name}.git"), None, None)
            .unwrap();
    }
    project
        .set_repo_location("backend", Some(remote_location("vm")))
        .unwrap();
    project
        .set_repo_location("agent", Some(remote_location("vm")))
        .unwrap();
    project
        .set_workspace_location("feat-login", Some(remote_location("other-vm")))
        .unwrap();

    assert_eq!(
        project.locations(),
        [
            Location::Local,
            remote_location("vm"),
            remote_location("other-vm")
        ]
    );
}

#[test]
fn location_of_an_unknown_repo_is_refused() {
    let mut project = Project::new("deeptree", "main").unwrap();
    let err = project
        .set_repo_location("nope", Some(Location::Local))
        .unwrap_err();
    assert!(matches!(err, Error::RepoNotFound(_)), "{err:?}");
}

fn project_with_backend() -> Project {
    let mut project = Project::new("deeptree", "main").unwrap();
    project
        .add_repo("git@github.com:org/backend.git", None, None)
        .unwrap();
    project
}

#[test]
fn setup_and_run_commands_of_a_repo_can_be_set_and_cleared() {
    let mut project = project_with_backend();
    assert_eq!(project.repos["backend"].setup, None);
    assert_eq!(project.effective_run("backend", "feat-login"), None);

    project.set_repo_setup("backend", Some("uv sync")).unwrap();
    project
        .set_repo_run("backend", Some("uv run main.py"))
        .unwrap();
    assert_eq!(project.repos["backend"].setup.as_deref(), Some("uv sync"));
    assert_eq!(
        project.effective_run("backend", "feat-login"),
        Some("uv run main.py")
    );

    project.set_repo_setup("backend", None).unwrap();
    project.set_repo_run("backend", None).unwrap();
    assert_eq!(project.repos["backend"].setup, None);
    assert_eq!(project.effective_run("backend", "feat-login"), None);
}

#[test]
fn run_command_of_a_workspace_overrides_the_one_of_the_repo() {
    let mut project = project_with_backend();
    project
        .set_repo_run("backend", Some("PORT=9005 uv run main.py"))
        .unwrap();

    project
        .set_workspace_run("feat-login", "backend", Some("PORT=9008 uv run main.py"))
        .unwrap();
    assert_eq!(
        project.effective_run("backend", "feat-login"),
        Some("PORT=9008 uv run main.py")
    );
    assert_eq!(
        project.effective_run("backend", "feat-other"),
        Some("PORT=9005 uv run main.py")
    );

    project
        .set_workspace_run("feat-login", "backend", None)
        .unwrap();
    assert_eq!(
        project.effective_run("backend", "feat-login"),
        Some("PORT=9005 uv run main.py")
    );
    assert!(project.workspaces.is_empty());
}

#[test]
fn blank_commands_are_rejected_and_change_nothing() {
    let mut project = project_with_backend();
    let before = project.clone();

    for command in ["", "  \n"] {
        let errors = [
            project.set_repo_setup("backend", Some(command)),
            project.set_repo_run("backend", Some(command)),
            project.set_workspace_run("feat-login", "backend", Some(command)),
        ];
        for err in errors.map(Result::unwrap_err) {
            assert!(matches!(err, Error::InvalidName { .. }), "{err:?}");
        }
    }
    assert_eq!(project, before);
}

#[test]
fn commands_of_an_unknown_repo_are_refused() {
    let mut project = project_with_backend();

    let errors = [
        project.set_repo_setup("nope", Some("uv sync")),
        project.set_repo_run("nope", Some("uv run main.py")),
        project.set_workspace_run("feat-login", "nope", Some("uv run main.py")),
    ];
    for err in errors.map(Result::unwrap_err) {
        assert!(matches!(err, Error::RepoNotFound(_)), "{err:?}");
    }
    assert!(project.workspaces.is_empty());
}

#[test]
fn a_workspace_keeps_its_run_commands_when_its_location_is_cleared() {
    let mut project = project_with_backend();
    project
        .set_workspace_location("feat-login", Some(remote_location("vm")))
        .unwrap();
    project
        .set_workspace_run("feat-login", "backend", Some("uv run main.py"))
        .unwrap();

    project.set_workspace_location("feat-login", None).unwrap();
    assert_eq!(
        project.effective_run("backend", "feat-login"),
        Some("uv run main.py")
    );
    assert_eq!(
        project.effective_location(&project.repos["backend"], Some("feat-login")),
        &Location::Local
    );

    project.forget_workspace("feat-login");
    assert!(project.workspaces.is_empty());
}

#[test]
fn removing_a_repo_forgets_its_run_commands_in_the_workspaces() {
    let mut project = project_with_backend();
    project
        .set_workspace_run("feat-login", "backend", Some("uv run main.py"))
        .unwrap();

    project.remove_repo("backend").unwrap();

    assert!(project.workspaces.is_empty());
}

fn reviewed(repo: &str, branch: &str) -> HashMap<String, String> {
    HashMap::from([(repo.to_string(), branch.to_string())])
}

#[test]
fn a_review_is_saved_with_the_branch_of_its_workspace_and_of_each_repo() {
    let mut project = project_with_backend();
    let login = reviewed("backend", "feat/login");

    project
        .set_review("pr-backend-12", "pr-backend-12", &login)
        .unwrap();
    assert_eq!(
        project.review("pr-backend-12"),
        Some(("pr-backend-12", login.clone()))
    );
    assert_eq!(project.review("feat-login"), None);

    // A run command of its own comes and goes without touching the review.
    project
        .set_workspace_run("pr-backend-12", "backend", Some("uv run main.py"))
        .unwrap();
    project
        .set_workspace_run("pr-backend-12", "backend", None)
        .unwrap();
    assert_eq!(
        project.review("pr-backend-12"),
        Some(("pr-backend-12", login))
    );

    project.clear_review("pr-backend-12");
    assert!(project.workspaces.is_empty());
}

#[test]
fn a_review_needs_known_repos_and_valid_branches() {
    let mut project = project_with_backend();

    assert!(matches!(
        project.set_review("pr-1", "pr-1", &reviewed("nope", "feat/login")),
        Err(Error::RepoNotFound(repo)) if repo == "nope"
    ));
    assert!(matches!(
        project.set_review("pr-1", "pr-1", &reviewed("backend", "bad branch")),
        Err(Error::InvalidName { kind: "branch", .. })
    ));
    assert!(project.workspaces.is_empty());
}

fn env_of(project: &Project, key: &str, workspace: &str) -> Vec<(String, String)> {
    project
        .effective_env(key, workspace)
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

#[test]
fn env_of_a_repo_is_set_and_removed() {
    let mut project = project_with_backend();

    project
        .set_repo_env("backend", "PORT", Some("9008"))
        .unwrap();
    assert_eq!(
        env_of(&project, "backend", "any"),
        pairs(&[("PORT", "9008")])
    );

    project.set_repo_env("backend", "PORT", None).unwrap();
    assert!(project.effective_env("backend", "any").is_empty());
    let yaml = serde_saphyr::to_string(&project).unwrap();
    assert!(!yaml.contains("env:"), "{yaml}");
}

#[test]
fn env_of_a_workspace_overrides_and_extends_the_one_of_the_repo() {
    let mut project = project_with_backend();
    project
        .set_repo_env("backend", "PORT", Some("9008"))
        .unwrap();
    project
        .set_repo_env("backend", "DB_PORT", Some("5432"))
        .unwrap();

    project
        .set_workspace_env("feat-login", "backend", "PORT", Some("9009"))
        .unwrap();
    project
        .set_workspace_env("feat-login", "backend", "EXTRA", Some("1"))
        .unwrap();

    assert_eq!(
        env_of(&project, "backend", "feat-login"),
        pairs(&[("PORT", "9009"), ("DB_PORT", "5432"), ("EXTRA", "1")])
    );
    assert_eq!(
        env_of(&project, "backend", "feat-other"),
        pairs(&[("PORT", "9008"), ("DB_PORT", "5432")])
    );
}

#[test]
fn removing_the_last_env_of_a_workspace_drops_its_entry() {
    let mut project = project_with_backend();
    project
        .set_workspace_env("feat-login", "backend", "PORT", Some("9009"))
        .unwrap();

    project
        .set_workspace_env("feat-login", "backend", "PORT", None)
        .unwrap();

    assert!(project.workspaces.is_empty());
}

#[test]
fn a_workspace_with_env_keeps_its_entry_when_its_run_command_is_removed() {
    let mut project = project_with_backend();
    project
        .set_workspace_run("feat-login", "backend", Some("uv run main.py"))
        .unwrap();
    project
        .set_workspace_env("feat-login", "backend", "PORT", Some("9009"))
        .unwrap();

    project
        .set_workspace_run("feat-login", "backend", None)
        .unwrap();

    assert_eq!(
        env_of(&project, "backend", "feat-login"),
        pairs(&[("PORT", "9009")])
    );
}

fn env_refusal(result: octotask_core::Result<()>) -> (String, &'static str) {
    match result {
        Err(Error::InvalidEnv { name, reason, .. }) => (name, reason),
        other => panic!("expected InvalidEnv, got {other:?}"),
    }
}

#[test]
fn env_names_and_values_are_checked() {
    let mut project = project_with_backend();
    let before = project.clone();
    let bad_name = "must start with a letter or `_` and contain only letters, digits and `_`";

    for name in ["1PORT", "MY-PORT", ""] {
        let refusals = [
            project.set_repo_env("backend", name, Some("1")),
            project.set_workspace_env("feat-login", "backend", name, Some("1")),
        ];
        for refusal in refusals {
            assert_eq!(env_refusal(refusal), (name.to_string(), bad_name));
        }
    }
    let bad_values = [
        ("a\nb", "must not contain a newline or a NUL character"),
        ("a\0b", "must not contain a newline or a NUL character"),
        ("{{ workspace.name", "has `{{` without `}}`"),
        ("{{ repo.backend.env.PORT }}", "uses an unknown variable"),
        ("{{ repos.backend.PORT }}", "uses an unknown variable"),
        ("{{}}", "uses an unknown variable"),
    ];
    for (value, reason) in bad_values {
        let refusals = [
            project.set_repo_env("backend", "PORT", Some(value)),
            project.set_workspace_env("feat-login", "backend", "PORT", Some(value)),
        ];
        for refusal in refusals {
            assert_eq!(
                env_refusal(refusal),
                ("PORT".to_string(), reason),
                "{value:?}"
            );
        }
    }
    assert_eq!(project, before);
}

#[test]
fn env_values_may_use_every_known_variable_and_plain_braces() {
    let mut project = project_with_backend();

    for value in [
        "{{project.name}}-{{ workspace.name }}-{{workspace.slug}}",
        "http://localhost:{{repos.backend.env.PORT}}",
        "{{ repos.nope.env.LATER }}",
        "{a} } {",
        "",
    ] {
        project
            .set_repo_env("backend", "VALUE", Some(value))
            .unwrap_or_else(|error| panic!("{value:?}: {error}"));
    }
}

#[test]
fn env_of_an_unknown_repo_is_refused() {
    let mut project = project_with_backend();

    let errors = [
        project.set_repo_env("nope", "PORT", Some("1")),
        project.set_workspace_env("feat-login", "nope", "PORT", Some("1")),
    ];
    for err in errors.map(Result::unwrap_err) {
        assert!(matches!(err, Error::RepoNotFound(_)), "{err:?}");
    }
    assert!(project.workspaces.is_empty());
}

#[test]
fn removing_a_repo_drops_its_env_in_every_workspace() {
    let mut project = project_with_backend();
    project
        .set_workspace_env("feat-login", "backend", "PORT", Some("9009"))
        .unwrap();

    project.remove_repo("backend").unwrap();

    assert!(project.workspaces.is_empty());
}
