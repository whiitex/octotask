use octotask_core::{Error, Project};

fn shop() -> Project {
    let mut project = Project::new("shop", "main").unwrap();
    for url in [
        "git@github.com:acme/backend.git",
        "git@github.com:acme/frontend.git",
    ] {
        project.add_repo(url, None, None).unwrap();
    }
    project.set_alias("backend", "api").unwrap();
    project
}

fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

#[test]
fn a_value_without_templates_is_passed_as_it_is() {
    let mut project = shop();
    project
        .set_repo_env("backend", "URL", Some("http://x/?a=b {c} $HOME"))
        .unwrap();

    assert_eq!(
        project.resolved_env("backend", "feat-login").unwrap(),
        pairs(&[("URL", "http://x/?a=b {c} $HOME")])
    );
}

#[test]
fn project_and_workspace_variables_are_replaced() {
    let mut project = shop();
    project
        .set_repo_env(
            "backend",
            "NAME",
            Some("{{project.name}}-{{ workspace.name }}"),
        )
        .unwrap();
    project
        .set_repo_env("backend", "SLUG", Some("{{workspace.slug}}"))
        .unwrap();

    assert_eq!(
        project.resolved_env("backend", "feat-Login.v2").unwrap(),
        pairs(&[("NAME", "shop-feat-Login.v2"), ("SLUG", "feat-login-v2")])
    );
}

#[test]
fn a_repo_reads_the_env_of_another_by_key_or_alias() {
    let mut project = shop();
    project
        .set_repo_env("backend", "PORT", Some("9008"))
        .unwrap();
    project
        .set_repo_env(
            "frontend",
            "BY_KEY",
            Some("http://localhost:{{repos.backend.env.PORT}}"),
        )
        .unwrap();
    project
        .set_repo_env("frontend", "BY_ALIAS", Some("{{ repos.api.env.PORT }}"))
        .unwrap();
    project
        .set_workspace_env("feat-login", "backend", "PORT", Some("9009"))
        .unwrap();

    assert_eq!(
        project.resolved_env("frontend", "feat-other").unwrap(),
        pairs(&[("BY_KEY", "http://localhost:9008"), ("BY_ALIAS", "9008")])
    );
    assert_eq!(
        project.resolved_env("frontend", "feat-login").unwrap(),
        pairs(&[("BY_KEY", "http://localhost:9009"), ("BY_ALIAS", "9009")])
    );
}

#[test]
fn chains_are_resolved() {
    let mut project = shop();
    project
        .set_repo_env("backend", "HOST", Some("localhost"))
        .unwrap();
    project
        .set_repo_env(
            "backend",
            "URL",
            Some("http://{{repos.backend.env.HOST}}:9008"),
        )
        .unwrap();
    project
        .set_repo_env("frontend", "API", Some("{{repos.backend.env.URL}}/api"))
        .unwrap();

    assert_eq!(
        project.resolved_env("frontend", "feat-login").unwrap(),
        pairs(&[("API", "http://localhost:9008/api")])
    );
}

fn undefined(project: &Project, key: &str, workspace: &str) -> (String, String, String) {
    match project.resolved_env(key, workspace) {
        Err(Error::UndefinedVariable {
            repo,
            name,
            variable,
        }) => (repo, name, variable),
        other => panic!("expected UndefinedVariable, got {other:?}"),
    }
}

#[test]
fn an_undefined_variable_is_an_error() {
    let mut project = shop();
    project
        .set_workspace_env("feat-login", "backend", "PORT", Some("9009"))
        .unwrap();
    let cases = [
        "{{ repos.backend.env.NOPE }}",
        "{{repos.nope.env.PORT}}",
        "{{repos.backend.env.PORT}}",
    ];

    for value in cases {
        project
            .set_repo_env("frontend", "API", Some(value))
            .unwrap();
        let variable = value.trim_matches(['{', '}', ' ']).to_string();
        assert_eq!(
            undefined(&project, "frontend", "feat-other"),
            ("frontend".to_string(), "API".to_string(), variable),
            "{value}"
        );
    }
}

#[test]
fn an_undefined_variable_further_down_a_chain_names_where_it_is() {
    let mut project = shop();
    project
        .set_repo_env("backend", "URL", Some("{{repos.backend.env.NOPE}}"))
        .unwrap();
    project
        .set_repo_env("frontend", "API", Some("{{repos.backend.env.URL}}"))
        .unwrap();

    assert_eq!(
        undefined(&project, "frontend", "feat-login"),
        (
            "backend".to_string(),
            "URL".to_string(),
            "repos.backend.env.NOPE".to_string()
        )
    );
}

#[test]
fn a_loop_is_an_error() {
    let mut project = shop();
    project
        .set_repo_env("backend", "A", Some("{{repos.backend.env.B}}"))
        .unwrap();
    project
        .set_repo_env("backend", "B", Some("x{{repos.api.env.A}}"))
        .unwrap();

    let error = project.resolved_env("backend", "feat-login").unwrap_err();

    assert!(
        matches!(&error, Error::EnvCycle(chain) if chain == &["backend.A", "backend.B", "backend.A"]),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "env values refer to each other in a loop: backend.A -> backend.B -> backend.A"
    );
}

#[test]
fn errors_never_show_a_value() {
    let mut project = shop();
    project
        .set_repo_env(
            "backend",
            "TOKEN",
            Some("s3cr3t {{repos.backend.env.NOPE}}"),
        )
        .unwrap();

    let error = project.resolved_env("backend", "feat-login").unwrap_err();

    assert_eq!(
        error.to_string(),
        "env `TOKEN` of repo `backend` uses `{{ repos.backend.env.NOPE }}`, which is not defined"
    );
    let document = serde_json::to_string(&error).unwrap();
    assert!(!document.contains("s3cr3t"), "{document}");
    for refused in [
        project.set_repo_env("backend", "BAD-NAME", Some("s3cr3t")),
        project.set_repo_env("backend", "BAD", Some("s3cr3t {{ nope }}")),
    ] {
        let error = refused.unwrap_err();
        assert!(!error.to_string().contains("s3cr3t"), "{error}");
        assert!(!serde_json::to_string(&error).unwrap().contains("s3cr3t"));
    }
}

#[test]
fn a_repo_with_a_dot_in_its_key_or_alias_can_be_read() {
    let mut project = shop();
    project
        .add_repo("git@github.com:acme/site.io.git", None, None)
        .unwrap();
    project.set_alias("site.io", "web.site").unwrap();
    project
        .set_repo_env("site.io", "PORT", Some("4000"))
        .unwrap();
    project
        .set_repo_env("backend", "BY_KEY", Some("{{ repos.site.io.env.PORT }}"))
        .unwrap();
    project
        .set_repo_env("backend", "BY_ALIAS", Some("{{repos.web.site.env.PORT}}"))
        .unwrap();

    assert_eq!(
        project.resolved_env("backend", "feat-login").unwrap(),
        pairs(&[("BY_KEY", "4000"), ("BY_ALIAS", "4000")])
    );
}
