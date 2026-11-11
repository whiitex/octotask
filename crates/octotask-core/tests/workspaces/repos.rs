use std::fs;

use octotask_core::{
    Error, NewRepo, create_workspace, fetch_repos, register_repo, unregister_repo,
};

use super::{Fixture, PROJECT, commit_on, git, head, refuse_missing_base};

#[test]
fn register_repo_clones_the_remote_into_a_bare_cache() {
    let fx = Fixture::new();
    let url = fx.remote("frontend");

    let key = fx.register(&url);

    assert_eq!(key, "frontend");
    assert_eq!(fx.project().repos["frontend"].url, url);
    let cache = fx.cache_dir("frontend");
    assert_eq!(git(&cache, &["rev-parse", "--is-bare-repository"]), "true");
    assert_eq!(
        head(&cache, "refs/remotes/origin/main"),
        head(&fx.remote_dir("frontend"), "main")
    );
    // Remote branches must not be copied as local ones: local branches belong to workspaces.
    assert_eq!(git(&cache, &["branch", "--list"]), "");
}

#[test]
fn register_repo_with_unreachable_url_registers_nothing() {
    let fx = Fixture::new();
    let url = fx.remote_dir("missing").display().to_string();

    let err = register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &url,
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap_err();

    assert!(matches!(err, Error::CommandFailed { .. }), "{err:?}");
    assert!(fx.project().repos.is_empty());
    let leftovers: Vec<_> = fs::read_dir(fx.cache_dir("missing").parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn register_repo_rejects_a_duplicate_key_and_keeps_the_first_cache() {
    let fx = Fixture::new();
    let first = fx.remote("frontend");
    let second = fx.remote("other/frontend");
    fx.register(&first);

    let err = register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &second,
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap_err();

    assert!(matches!(err, Error::DuplicateRepo { .. }), "{err:?}");
    assert_eq!(
        git(&fx.cache_dir("frontend"), &["remote", "get-url", "origin"]),
        first
    );
}

#[test]
fn fetch_repos_brings_new_commits_into_every_cache() {
    let fx = Fixture::new();
    for name in ["frontend", "backend"] {
        fx.register(&fx.remote(name));
        commit_on(&fx.remote_dir(name), "main", "second");
    }

    let report = fetch_repos(&fx.home(), &fx.project());

    let keys: Vec<_> = report.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["frontend", "backend"]);
    for (key, result) in &report {
        result.as_ref().unwrap();
        assert_eq!(
            head(&fx.cache_dir(key), "refs/remotes/origin/main"),
            head(&fx.remote_dir(key), "main")
        );
    }
}

#[test]
fn fetch_repos_clones_a_missing_cache() {
    let fx = Fixture::new();
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &fx.remote("frontend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    fs::remove_dir_all(fx.cache_dir("frontend")).unwrap();

    let report = fetch_repos(&fx.home(), &fx.project());

    report[0].1.as_ref().unwrap();
    assert!(fx.cache_dir("frontend").is_dir());
}

#[test]
fn fetch_repos_continues_after_a_failing_repo() {
    let fx = Fixture::new();
    for name in ["frontend", "backend"] {
        fx.register(&fx.remote(name));
    }
    fs::remove_dir_all(fx.remote_dir("frontend")).unwrap();
    commit_on(&fx.remote_dir("backend"), "main", "second");

    let report = fetch_repos(&fx.home(), &fx.project());

    assert!(report[0].1.is_err());
    report[1].1.as_ref().unwrap();
    assert_eq!(
        head(&fx.cache_dir("backend"), "refs/remotes/origin/main"),
        head(&fx.remote_dir("backend"), "main")
    );
}

#[test]
fn fetch_repos_follows_a_url_changed_in_the_project_file() {
    let fx = Fixture::new();
    let old = fx.remote("old");
    let new = fx.remote("new");
    commit_on(&fx.remote_dir("new"), "main", "only in new");
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &old,
            key: Some("app"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    fx.registry()
        .update_project(PROJECT, |project| {
            project.repos["app"].url = new.clone();
            Ok(())
        })
        .unwrap();

    let report = fetch_repos(&fx.home(), &fx.project());

    report[0].1.as_ref().unwrap();
    assert_eq!(
        head(&fx.cache_dir("app"), "refs/remotes/origin/main"),
        head(&fx.remote_dir("new"), "main")
    );
}

#[test]
fn unregister_repo_removes_the_entry_and_the_cache() {
    let fx = Fixture::new();
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &fx.remote("frontend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();

    let key = unregister_repo(&fx.registry(), PROJECT, "frontend").unwrap();

    assert_eq!(key, "frontend");
    assert!(fx.project().repos.is_empty());
    assert!(!fx.cache_dir("frontend").exists());
    assert!(!fx.cache_dir("frontend").with_extension("lock").exists());
}

#[test]
fn unregister_repo_used_by_a_workspace_is_refused() {
    let fx = Fixture::new();
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &fx.remote("frontend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap();

    let err = unregister_repo(&fx.registry(), PROJECT, "frontend").unwrap_err();

    assert!(
        matches!(&err, Error::RepoInUse { workspaces, .. } if workspaces == &["feat-login"]),
        "{err:?}"
    );
    assert!(fx.project().repos.contains_key("frontend"));
    assert!(fx.cache_dir("frontend").is_dir());
}

#[test]
fn unregister_repo_that_is_not_registered_fails() {
    let fx = Fixture::new();

    let err = unregister_repo(&fx.registry(), PROJECT, "frontend").unwrap_err();

    assert!(matches!(err, Error::RepoNotFound(_)), "{err:?}");
}

#[test]
fn register_repo_stores_the_alias_and_unregister_repo_accepts_it() {
    let fx = Fixture::new();
    let url = fx.remote("DeepTreeDashboard");

    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &url,
            alias: Some("frontend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    assert_eq!(
        fx.project().repos["DeepTreeDashboard"].alias.as_deref(),
        Some("frontend")
    );

    let key = unregister_repo(&fx.registry(), PROJECT, "frontend").unwrap();
    assert_eq!(key, "DeepTreeDashboard");
    assert!(fx.project().repos.is_empty());
    assert!(!fx.cache_dir("DeepTreeDashboard").exists());
}

#[test]
fn register_repo_checks_the_alias_before_cloning() {
    let fx = Fixture::new();
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &fx.remote("frontend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    let url = fx.remote("backend");

    let err = register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &url,
            alias: Some("frontend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap_err();

    assert!(matches!(err, Error::DuplicateAlias { .. }), "{err:?}");
    assert!(!fx.cache_dir("backend").exists());
}

#[test]
fn register_repo_asks_before_using_a_base_missing_on_the_remote() {
    let fx = Fixture::new();
    let url = fx.remote("frontend");
    let repo = NewRepo {
        url: &url,
        base: Some("develop"),
        ..Default::default()
    };
    let mut asked = Vec::new();

    let err = register_repo(&fx.registry(), PROJECT, &repo, |key, base| {
        asked.push((key.to_string(), base.to_string()));
        false
    })
    .unwrap_err();

    assert_eq!(asked, [("frontend".to_string(), "develop".to_string())]);
    assert!(
        matches!(&err, Error::BaseNotFound { repo, base } if repo == "frontend" && base == "develop"),
        "{err:?}"
    );
    assert!(fx.project().repos.is_empty());
    assert!(!fx.cache_dir("frontend").exists());
    assert!(!fx.cache_dir("frontend").with_extension("lock").exists());

    register_repo(&fx.registry(), PROJECT, &repo, |_, _| true).unwrap();
    assert_eq!(
        fx.project().repos["frontend"].base.as_deref(),
        Some("develop")
    );
}

#[test]
fn register_repo_does_not_ask_when_the_base_exists() {
    let fx = Fixture::new();
    let url = fx.remote("frontend");
    let repo = NewRepo {
        url: &url,
        ..Default::default()
    };

    register_repo(&fx.registry(), PROJECT, &repo, |_, _| {
        panic!("the base exists: nothing to ask")
    })
    .unwrap();
}

#[test]
fn register_repo_clears_the_leftovers_of_an_interrupted_clone() {
    let fx = Fixture::new();
    let caches = fx.cache_dir("frontend").parent().unwrap().to_path_buf();
    let leftover = caches.join("frontend.tmp-4242");
    let unrelated = caches.join("frontend.tmp-4242.git");
    std::fs::create_dir_all(leftover.join("objects")).unwrap();
    std::fs::create_dir_all(&unrelated).unwrap();

    fx.register(&fx.remote("frontend"));

    assert!(!leftover.exists());
    assert!(unrelated.is_dir());
    assert!(fx.cache_dir("frontend").is_dir());
}
