use octotask_core::{Error, Project, repo_key_from_url};

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
    assert!(matches!(err, Error::DuplicateRepo(k) if k == "frontend"));
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
