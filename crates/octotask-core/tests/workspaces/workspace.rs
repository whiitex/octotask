use std::fs;

use octotask_core::{
    Error, NewRepo, PullOutcome, create_workspace, locate_workspace, pull_workspace, register_repo,
    remove_workspace, status, workspace_path,
};

use super::{Fixture, PROJECT, commit_here, commit_on, git, head, refuse_missing_base};

fn fixture_with_repos(names: &[&str]) -> Fixture {
    let fx = Fixture::new();
    for name in names {
        fx.register(&fx.remote(name));
    }
    fx
}

#[test]
fn create_workspace_adds_one_worktree_per_repo_on_a_new_branch_from_the_latest_base() {
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
    let backend = fx.remote("backend");
    commit_on(&fx.remote_dir("backend"), "develop", "on develop");
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &backend,
            base: Some("develop"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    // Pushed after the clone: only a fetch at creation time can see them.
    commit_on(&fx.remote_dir("frontend"), "main", "newer main");
    commit_on(&fx.remote_dir("backend"), "develop", "newer develop");

    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;

    assert_eq!(path, fx.workspace_dir("feat-login"));
    for (repo, base) in [("frontend", "main"), ("backend", "develop")] {
        let worktree = path.join(repo);
        assert_eq!(git(&worktree, &["branch", "--show-current"]), "feat/login");
        assert_eq!(head(&worktree, "HEAD"), head(&fx.remote_dir(repo), base));
    }
}

#[test]
fn create_workspace_refuses_a_branch_that_exists_on_a_remote_and_creates_nothing() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    commit_on(
        &fx.remote_dir("backend"),
        "feat/login",
        "someone else's work",
    );

    let err = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap_err();

    assert!(
        matches!(&err, Error::BranchExists { branch, repos }
            if branch == "feat/login" && repos == &["backend"]),
        "{err:?}"
    );
    assert!(!fx.workspace_dir("feat-login").exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn create_workspace_restores_a_branch_left_in_the_cache_with_its_commits() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    let local_head = head(&path.join("frontend"), "HEAD");
    fs::remove_dir_all(&path).unwrap();

    let workspace = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap();

    assert_eq!(workspace.restored, ["frontend"]);
    assert!(workspace.created.is_empty() && workspace.kept.is_empty());
    assert_eq!(head(&path.join("frontend"), "HEAD"), local_head);
    assert_eq!(
        git(&path.join("frontend"), &["branch", "--show-current"]),
        "feat/login"
    );
}

#[test]
fn create_workspace_adds_the_worktrees_missing_from_an_existing_workspace() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    let local_head = head(&path.join("frontend"), "HEAD");
    fx.register(&fx.remote("backend"));

    let workspace = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap();

    assert_eq!(workspace.created, ["backend"]);
    assert_eq!(workspace.kept, ["frontend"]);
    assert!(workspace.restored.is_empty());
    assert_eq!(head(&path.join("frontend"), "HEAD"), local_head);
    assert_eq!(
        git(&path.join("backend"), &["branch", "--show-current"]),
        "feat/login"
    );
}

#[test]
fn create_workspace_lets_a_plain_git_push_publish_the_new_branch() {
    let fx = fixture_with_repos(&["frontend"]);
    let worktree = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path
        .join("frontend");
    commit_here(&worktree, "local work");

    git(&worktree, &["push", "--quiet"]);

    assert_eq!(
        head(&fx.remote_dir("frontend"), "feat/login"),
        head(&worktree, "HEAD")
    );
    assert_eq!(
        git(&worktree, &["rev-parse", "--abbrev-ref", "@{upstream}"]),
        "origin/feat/login"
    );
}

#[test]
fn create_workspace_refuses_an_existing_workspace() {
    let fx = fixture_with_repos(&["frontend"]);
    create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap();

    let err = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap_err();

    assert!(matches!(err, Error::WorkspaceExists { .. }), "{err:?}");
}

#[test]
fn create_workspace_needs_at_least_one_repo() {
    let fx = Fixture::new();

    let err = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap_err();

    assert!(matches!(err, Error::NoRepos(_)), "{err:?}");
}

#[test]
fn create_workspace_rejects_an_invalid_branch_name() {
    let fx = fixture_with_repos(&["frontend"]);

    let err = create_workspace(&fx.home(), &fx.project(), "feat login").unwrap_err();

    assert!(matches!(err, Error::InvalidName { .. }), "{err:?}");
}

#[test]
fn create_workspace_reports_a_base_branch_missing_on_the_remote() {
    let fx = Fixture::new();
    let url = fx.remote("frontend");
    let repo = NewRepo {
        url: &url,
        base: Some("develop"),
        ..Default::default()
    };
    register_repo(&fx.registry(), PROJECT, &repo, |_, _| true).unwrap();

    let err = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap_err();

    assert!(
        matches!(&err, Error::BaseNotFound { repo, base } if repo == "frontend" && base == "develop"),
        "{err:?}"
    );
    assert!(!fx.workspace_dir("feat-login").exists());
}

#[test]
fn create_workspace_undoes_the_worktrees_already_added_when_a_later_repo_fails() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    // git cannot register a worktree when `worktrees` is a file.
    fs::write(fx.cache_dir("backend").join("worktrees"), "").unwrap();

    let err = create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap_err();

    assert!(matches!(err, Error::CommandFailed { .. }), "{err:?}");
    assert!(!fx.workspace_dir("feat-login").exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn workspace_path_finds_an_existing_workspace_by_branch() {
    let fx = fixture_with_repos(&["frontend"]);
    let created = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;

    assert_eq!(
        workspace_path(&fx.home(), PROJECT, "feat/login").unwrap(),
        created
    );
    let err = workspace_path(&fx.home(), PROJECT, "feat/other").unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceNotFound(branch) if branch == "feat/other"),
        "{err:?}"
    );
}

#[test]
fn locate_workspace_recognises_any_directory_inside_a_workspace() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    let nested = path.join("frontend").join("src");
    fs::create_dir(&nested).unwrap();
    let expected = Some((PROJECT.to_string(), "feat-login".to_string()));

    assert_eq!(locate_workspace(&fx.home(), &nested), expected);
    assert_eq!(locate_workspace(&fx.home(), &path), expected);
    assert_eq!(locate_workspace(&fx.home(), &fx.home_dir()), None);
    assert_eq!(locate_workspace(&fx.home(), path.parent().unwrap()), None);
}

#[test]
fn pull_workspace_fast_forwards_each_worktree_to_its_remote_branch() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    commit_on(
        &fx.remote_dir("frontend"),
        "feat/login",
        "pushed by a teammate",
    );

    let report = pull_workspace(&fx.home(), &fx.project(), "feat-login").unwrap();

    let outcomes: Vec<_> = report
        .iter()
        .map(|(key, result)| (key.as_str(), *result.as_ref().unwrap()))
        .collect();
    assert_eq!(
        outcomes,
        [
            ("frontend", PullOutcome::Updated),
            ("backend", PullOutcome::NoRemoteBranch),
        ]
    );
    assert_eq!(
        head(&path.join("frontend"), "HEAD"),
        head(&fx.remote_dir("frontend"), "feat/login")
    );
}

#[test]
fn pull_workspace_leaves_up_to_date_and_diverged_worktrees_alone() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    git(
        &fx.remote_dir("frontend"),
        &["branch", "feat/login", "main"],
    );
    commit_on(&fx.remote_dir("backend"), "feat/login", "remote work");
    commit_here(&path.join("backend"), "local work");
    let local_head = head(&path.join("backend"), "HEAD");

    let report = pull_workspace(&fx.home(), &fx.project(), "feat-login").unwrap();

    let outcomes: Vec<_> = report
        .iter()
        .map(|(_, result)| *result.as_ref().unwrap())
        .collect();
    assert_eq!(outcomes, [PullOutcome::UpToDate, PullOutcome::Diverged]);
    assert_eq!(head(&path.join("backend"), "HEAD"), local_head);
}

#[test]
fn pull_workspace_skips_repos_without_a_worktree_or_a_branch() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    git(&path.join("frontend"), &["checkout", "--quiet", "--detach"]);
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &fx.remote("backend"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();

    let report = pull_workspace(&fx.home(), &fx.project(), "feat-login").unwrap();

    let outcomes: Vec<_> = report
        .iter()
        .map(|(_, result)| *result.as_ref().unwrap())
        .collect();
    assert_eq!(outcomes, [PullOutcome::Detached, PullOutcome::NoWorktree]);
}

#[test]
fn pull_workspace_fails_for_an_unknown_workspace() {
    let fx = fixture_with_repos(&["frontend"]);

    let err = pull_workspace(&fx.home(), &fx.project(), "feat-login").unwrap_err();

    assert!(matches!(err, Error::WorkspaceNotFound(_)), "{err:?}");
}

#[test]
fn status_reports_repos_and_the_checkouts_of_every_workspace() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    let login = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    create_workspace(&fx.home(), &fx.project(), "fix/typo").unwrap();
    commit_here(&login.join("backend"), "add login endpoint");

    let status = status(&fx.home(), &fx.project()).unwrap();

    assert_eq!(status.project, PROJECT);
    let repo = &status.repos[0];
    assert_eq!(
        (repo.key.as_str(), repo.base.as_str()),
        ("frontend", "main")
    );
    let base_commit = repo.commit.as_ref().unwrap();
    assert_eq!(base_commit.subject, "init");
    assert!(head(&fx.remote_dir("frontend"), "main").starts_with(&base_commit.sha));

    let names: Vec<_> = status.workspaces.iter().map(|w| w.name.as_str()).collect();
    assert_eq!(names, ["feat-login", "fix-typo"]);
    let workspace = &status.workspaces[0];
    assert_eq!(workspace.path, login);
    let checkout = &workspace.checkouts[1];
    assert_eq!(checkout.repo, "backend");
    assert_eq!(checkout.branch.as_deref(), Some("feat/login"));
    let commit = checkout.commit.as_ref().unwrap();
    assert_eq!(commit.subject, "add login endpoint");
    assert!(!commit.date.is_empty() && !commit.age.is_empty());
    assert_eq!(checkout.error, None);
}

#[test]
fn status_marks_a_repo_without_cache_as_not_fetched() {
    let fx = fixture_with_repos(&["frontend"]);
    fs::remove_dir_all(fx.cache_dir("frontend")).unwrap();

    let status = status(&fx.home(), &fx.project()).unwrap();

    assert!(status.repos[0].commit.is_none());
    assert!(!status.repos[0].cached);
    assert!(status.workspaces.is_empty());
    assert!(status.orphan_caches.is_empty());
}

#[test]
fn status_reports_aliases_and_caches_missing_from_the_project() {
    let fx = Fixture::new();
    let url = fx.remote("frontend");
    register_repo(
        &fx.registry(),
        PROJECT,
        &NewRepo {
            url: &url,
            alias: Some("web"),
            ..Default::default()
        },
        refuse_missing_base,
    )
    .unwrap();
    // A key renamed by hand in the project file leaves its old cache behind.
    fx.registry()
        .update_project(PROJECT, |project| {
            let repo = project.remove_repo("frontend")?;
            project.repos.insert("renamed".to_string(), repo);
            Ok(())
        })
        .unwrap();

    let status = status(&fx.home(), &fx.project()).unwrap();

    assert_eq!(status.repos[0].alias.as_deref(), Some("web"));
    assert_eq!(status.orphan_caches, [fx.cache_dir("frontend")]);
}

#[test]
fn status_tells_a_missing_base_branch_from_a_missing_cache() {
    let fx = Fixture::new();
    let url = fx.remote("frontend");
    let repo = NewRepo {
        url: &url,
        base: Some("develop"),
        ..Default::default()
    };
    register_repo(&fx.registry(), PROJECT, &repo, |_, _| true).unwrap();

    let status = status(&fx.home(), &fx.project()).unwrap();

    assert!(status.repos[0].cached);
    assert!(status.repos[0].commit.is_none());
}

#[test]
fn status_reports_a_broken_worktree_without_hiding_the_others() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap();
    // The worktree no longer knows its repository.
    fs::remove_dir_all(fx.cache_dir("backend").join("worktrees")).unwrap();

    let status = status(&fx.home(), &fx.project()).unwrap();

    let checkouts = &status.workspaces[0].checkouts;
    assert_eq!(checkouts[0].branch.as_deref(), Some("feat/login"));
    assert!(checkouts[0].commit.is_some() && checkouts[0].error.is_none());
    assert_eq!(checkouts[1].repo, "backend");
    assert!(checkouts[1].commit.is_none() && checkouts[1].branch.is_none());
    assert!(checkouts[1].error.is_some());
}

#[test]
fn remove_workspace_deletes_the_worktrees_and_their_branches() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;

    let name = remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap();

    assert_eq!(name, "feat-login");
    assert!(!path.exists());
    for repo in ["frontend", "backend"] {
        assert_eq!(git(&fx.cache_dir(repo), &["branch", "--list"]), "");
        assert_eq!(
            git(&fx.cache_dir(repo), &["worktree", "list"])
                .lines()
                .count(),
            1
        );
    }
    // The name is free again.
    create_workspace(&fx.home(), &fx.project(), "feat/login").unwrap();
}

#[test]
fn remove_workspace_refuses_to_lose_uncommitted_or_unpushed_work() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    fs::write(path.join("frontend").join("draft.txt"), "wip").unwrap();
    commit_here(&path.join("backend"), "local work");
    commit_here(&path.join("backend"), "more local work");

    let err = remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap_err();

    assert!(
        matches!(&err, Error::WorkspaceHasWork { name, reasons } if name == "feat-login"
        && reasons == &[
            "frontend: uncommitted changes",
            "backend: 2 unpushed commits on `feat/login`",
        ]),
        "{err:?}"
    );
    assert!(path.join("frontend").join("draft.txt").is_file());
    assert_eq!(
        git(&path.join("backend"), &["log", "-1", "--format=%s"]),
        "more local work"
    );
}

#[test]
fn remove_workspace_accepts_pushed_commits() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    git(&path.join("frontend"), &["push", "--quiet"]);

    remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap();

    assert!(!path.exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn remove_workspace_with_force_discards_local_work() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    fs::write(path.join("frontend").join("draft.txt"), "wip").unwrap();
    fs::write(path.join("notes.md"), "mine").unwrap();

    let err = remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceHasWork { reasons, .. }
            if reasons.contains(&"other files in the workspace: notes.md".to_string())),
        "{err:?}"
    );
    remove_workspace(&fx.home(), &fx.project(), "feat/login", true).unwrap();

    assert!(!path.exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn remove_workspace_cleans_up_after_a_folder_deleted_by_hand() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    fs::remove_dir_all(&path).unwrap();

    remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap();

    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
    let err = remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceNotFound(branch) if branch == "feat/login"),
        "{err:?}"
    );
}

#[test]
fn remove_workspace_with_force_removes_a_broken_worktree() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.home(), &fx.project(), "feat/login")
        .unwrap()
        .path;
    fs::remove_dir_all(fx.cache_dir("frontend").join("worktrees")).unwrap();

    let err = remove_workspace(&fx.home(), &fx.project(), "feat/login", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceHasWork { reasons, .. }
            if reasons == &["frontend: the worktree cannot be read"]),
        "{err:?}"
    );
    remove_workspace(&fx.home(), &fx.project(), "feat/login", true).unwrap();

    assert!(!path.exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}
