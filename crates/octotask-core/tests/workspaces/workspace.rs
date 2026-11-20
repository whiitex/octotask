use std::collections::HashMap;
use std::fs;

use octotask_core::{
    CheckoutOutcome, Error, Location, NewRepo, NewWorkspace, PullOutcome, Result,
    checkout_workspace, create_workspace, locate_workspace, pull_workspace, register_repo,
    remove_workspace, review_workspace, status, workspace_path,
};

use super::{
    Fixture, PROJECT, commit_here, commit_on, git, head, keep_project_base, refuse_missing_base,
};

fn fixture_with_repos(names: &[&str]) -> Fixture {
    let fx = Fixture::new();
    for name in names {
        fx.register(&fx.remote(name));
    }
    fx
}

/// A project whose backend has the branch `feat/login` and whose agent has `fix/prompt`,
/// as two pull requests would leave them. The frontend has neither.
fn fixture_with_two_reviewable_branches() -> Fixture {
    let fx = fixture_with_repos(&["frontend", "backend", "agent"]);
    commit_on(&fx.remote_dir("backend"), "feat/login", "add login");
    commit_on(&fx.remote_dir("agent"), "fix/prompt", "fix the prompt");
    fx
}

/// Reviews the workspace `pr-backend-12` with the given branch for each named repo.
fn review(fx: &Fixture, chosen: &[(&str, &str)]) -> Result<NewWorkspace> {
    let chosen: HashMap<String, String> = chosen
        .iter()
        .map(|(repo, branch)| (repo.to_string(), branch.to_string()))
        .collect();
    review_workspace(
        &fx.registry(),
        PROJECT,
        "pr-backend-12",
        &chosen,
        None,
        false,
    )
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
        keep_project_base,
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
        keep_project_base,
    )
    .unwrap();
    // Pushed after the clone: only a fetch at creation time can see them.
    commit_on(&fx.remote_dir("frontend"), "main", "newer main");
    commit_on(&fx.remote_dir("backend"), "develop", "newer develop");

    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
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

    let err = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

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
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    let local_head = head(&path.join("frontend"), "HEAD");
    fs::remove_dir_all(&path).unwrap();

    let workspace = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    assert_eq!(
        workspace.repos_with(CheckoutOutcome::Restored),
        ["frontend"]
    );
    assert!(
        workspace.repos_with(CheckoutOutcome::Created).is_empty()
            && workspace.repos_with(CheckoutOutcome::Kept).is_empty()
    );
    assert_eq!(head(&path.join("frontend"), "HEAD"), local_head);
    assert_eq!(
        git(&path.join("frontend"), &["branch", "--show-current"]),
        "feat/login"
    );
}

#[test]
fn create_workspace_adds_the_worktrees_missing_from_an_existing_workspace() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    let local_head = head(&path.join("frontend"), "HEAD");
    fx.register(&fx.remote("backend"));

    let workspace = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    assert_eq!(workspace.repos_with(CheckoutOutcome::Created), ["backend"]);
    assert_eq!(workspace.repos_with(CheckoutOutcome::Kept), ["frontend"]);
    assert!(workspace.repos_with(CheckoutOutcome::Restored).is_empty());
    assert_eq!(head(&path.join("frontend"), "HEAD"), local_head);
    assert_eq!(
        git(&path.join("backend"), &["branch", "--show-current"]),
        "feat/login"
    );
}

#[test]
fn create_workspace_lets_a_plain_git_push_publish_the_new_branch() {
    let fx = fixture_with_repos(&["frontend"]);
    let worktree = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
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
    create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    let err = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

    assert!(matches!(err, Error::WorkspaceExists { .. }), "{err:?}");
}

#[test]
fn create_workspace_needs_at_least_one_repo() {
    let fx = Fixture::new();

    let err = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

    assert!(matches!(err, Error::NoRepos(_)), "{err:?}");
}

#[test]
fn create_workspace_rejects_an_invalid_branch_name() {
    let fx = fixture_with_repos(&["frontend"]);

    let err = create_workspace(&fx.registry(), PROJECT, "feat login", None, false).unwrap_err();

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
    register_repo(
        &fx.registry(),
        PROJECT,
        &repo,
        |_, _| true,
        keep_project_base,
    )
    .unwrap();

    let err = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

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

    let err = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

    assert!(matches!(err, Error::CommandFailed { .. }), "{err:?}");
    assert!(!fx.workspace_dir("feat-login").exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn checkout_workspace_takes_the_remote_branch_where_it_exists_and_the_base_elsewhere() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    // Pushed after the clone: only a fetch at checkout time can see it.
    commit_on(&fx.remote_dir("backend"), "feat/login", "a teammate's work");

    let workspace = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    assert_eq!(workspace.path, fx.workspace_dir("feat-login"));
    assert_eq!(workspace.repos_with(CheckoutOutcome::Tracked), ["backend"]);
    assert_eq!(workspace.repos_with(CheckoutOutcome::Created), ["frontend"]);
    assert!(
        workspace.repos_with(CheckoutOutcome::Kept).is_empty()
            && workspace.repos_with(CheckoutOutcome::Restored).is_empty()
    );
    let backend = workspace.path.join("backend");
    assert_eq!(git(&backend, &["branch", "--show-current"]), "feat/login");
    assert_eq!(
        head(&backend, "HEAD"),
        head(&fx.remote_dir("backend"), "feat/login")
    );
    assert_eq!(
        git(&backend, &["rev-parse", "--abbrev-ref", "@{upstream}"]),
        "origin/feat/login"
    );
    let frontend = workspace.path.join("frontend");
    assert_eq!(git(&frontend, &["branch", "--show-current"]), "feat/login");
    assert_eq!(
        head(&frontend, "HEAD"),
        head(&fx.remote_dir("frontend"), "main")
    );
}

#[test]
fn checkout_workspace_refuses_a_branch_that_is_on_no_remote_and_creates_nothing() {
    let fx = fixture_with_repos(&["frontend", "backend"]);

    let err = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

    assert!(
        matches!(&err, Error::BranchNotFound(branch) if branch == "feat/login"),
        "{err:?}"
    );
    assert!(!fx.workspace_dir("feat-login").exists());
    for repo in ["frontend", "backend"] {
        assert_eq!(git(&fx.cache_dir(repo), &["branch", "--list"]), "");
    }
}

#[test]
fn checkout_workspace_returns_a_complete_workspace_without_the_network() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    commit_on(&fx.remote_dir("backend"), "feat/login", "a teammate's work");
    let path = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    for repo in ["frontend", "backend"] {
        fs::remove_dir_all(fx.remote_dir(repo)).unwrap();
    }

    let workspace = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    assert_eq!(workspace.path, path);
    assert_eq!(
        workspace.repos_with(CheckoutOutcome::Kept),
        ["frontend", "backend"]
    );
    assert!(
        workspace.repos_with(CheckoutOutcome::Tracked).is_empty()
            && workspace.repos_with(CheckoutOutcome::Created).is_empty()
    );
}

#[test]
fn checkout_workspace_adds_the_worktrees_missing_from_an_existing_workspace() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    let backend = fx.remote("backend");
    commit_on(&fx.remote_dir("backend"), "feat/login", "a teammate's work");
    fx.register(&backend);
    fx.register(&fx.remote("agent"));

    let workspace = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    assert_eq!(workspace.repos_with(CheckoutOutcome::Kept), ["frontend"]);
    assert_eq!(workspace.repos_with(CheckoutOutcome::Tracked), ["backend"]);
    assert_eq!(workspace.repos_with(CheckoutOutcome::Created), ["agent"]);
    assert_eq!(
        head(&path.join("backend"), "HEAD"),
        head(&fx.remote_dir("backend"), "feat/login")
    );
    assert_eq!(
        git(&path.join("agent"), &["branch", "--show-current"]),
        "feat/login"
    );
}

#[test]
fn checkout_workspace_undoes_the_branches_it_took_when_a_later_repo_fails() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    commit_on(
        &fx.remote_dir("frontend"),
        "feat/login",
        "a teammate's work",
    );
    // git cannot register a worktree when `worktrees` is a file.
    fs::write(fx.cache_dir("backend").join("worktrees"), "").unwrap();

    let err = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

    assert!(matches!(err, Error::CommandFailed { .. }), "{err:?}");
    assert!(!fx.workspace_dir("feat-login").exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn a_checked_out_branch_is_pulled_and_a_branch_published_later_is_followed() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    commit_on(&fx.remote_dir("backend"), "feat/login", "a teammate's work");
    let path = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    commit_on(&fx.remote_dir("backend"), "feat/login", "more work");
    commit_on(&fx.remote_dir("frontend"), "feat/login", "now here too");

    let report = pull_workspace(&fx.home(), &fx.project(), "feat-login").unwrap();

    let outcomes: Vec<_> = report
        .iter()
        .map(|repo| *repo.result.as_ref().unwrap())
        .collect();
    assert_eq!(outcomes, [PullOutcome::Updated, PullOutcome::Updated]);
    for repo in ["frontend", "backend"] {
        assert_eq!(
            head(&path.join(repo), "HEAD"),
            head(&fx.remote_dir(repo), "feat/login")
        );
        // So a plain `git pull` and `git status` work there too.
        assert_eq!(
            git(
                &path.join(repo),
                &["rev-parse", "--abbrev-ref", "@{upstream}"]
            ),
            "origin/feat/login"
        );
    }
}

#[test]
fn remove_workspace_accepts_a_workspace_that_was_only_checked_out() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    commit_on(&fx.remote_dir("backend"), "feat/login", "a teammate's work");
    let path = checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;

    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

    assert!(!path.exists());
    for repo in ["frontend", "backend"] {
        assert_eq!(git(&fx.cache_dir(repo), &["branch", "--list"]), "");
    }
}

#[test]
fn status_tells_whether_the_branch_of_a_checkout_is_on_the_remote() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    commit_on(&fx.remote_dir("backend"), "feat/login", "a teammate's work");
    checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();

    let status = status(&fx.home(), &fx.project()).unwrap();

    let on_remote: Vec<_> = status.workspaces[0]
        .checkouts
        .iter()
        .map(|checkout| (checkout.repo.as_str(), checkout.on_remote))
        .collect();
    assert_eq!(on_remote, [("frontend", false), ("backend", true)]);
}

#[test]
fn a_workspace_is_not_shared_by_two_branches_whose_names_give_the_same_folder() {
    let fx = fixture_with_repos(&["frontend"]);
    git(&fx.remote_dir("frontend"), &["branch", "feat/login"]);
    git(&fx.remote_dir("frontend"), &["branch", "feat-login"]);
    checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();
    fx.register(&fx.remote("backend"));

    let taken = |err: Error| {
        matches!(&err, Error::WorkspaceOfAnotherBranch { name, branch }
            if name == "feat-login" && branch == "feat/login")
    };
    let checked_out =
        checkout_workspace(&fx.registry(), PROJECT, "feat-login", None, false).unwrap_err();
    let created = create_workspace(&fx.registry(), PROJECT, "feat-login", None, false).unwrap_err();

    assert!(taken(checked_out));
    assert!(taken(created));
    assert!(!fx.workspace_dir("feat-login").join("backend").exists());
}

#[test]
fn review_workspace_gives_each_repo_its_branch_and_the_others_one_from_the_base() {
    let fx = fixture_with_two_reviewable_branches();

    let workspace = review(&fx, &[("backend", "feat/login"), ("agent", "fix/prompt")]).unwrap();

    assert_eq!(workspace.path, fx.workspace_dir("pr-backend-12"));
    assert_eq!(
        workspace.repos_with(CheckoutOutcome::Tracked),
        ["backend", "agent"]
    );
    assert_eq!(workspace.repos_with(CheckoutOutcome::Created), ["frontend"]);
    for (repo, branch) in [("backend", "feat/login"), ("agent", "fix/prompt")] {
        let worktree = workspace.path.join(repo);
        assert_eq!(git(&worktree, &["branch", "--show-current"]), branch);
        assert_eq!(head(&worktree, "HEAD"), head(&fx.remote_dir(repo), branch));
        assert_eq!(
            git(&worktree, &["rev-parse", "--abbrev-ref", "@{upstream}"]),
            format!("origin/{branch}")
        );
    }
    let frontend = workspace.path.join("frontend");
    assert_eq!(
        git(&frontend, &["branch", "--show-current"]),
        "pr-backend-12"
    );
    assert_eq!(
        head(&frontend, "HEAD"),
        head(&fx.remote_dir("frontend"), "main")
    );
}

#[test]
fn review_workspace_refuses_a_branch_its_repo_does_not_have_and_creates_nothing() {
    let fx = fixture_with_two_reviewable_branches();

    let err = review(&fx, &[("backend", "feat/login"), ("agent", "feat/login")]).unwrap_err();

    assert!(
        matches!(&err, Error::BranchNotOnRemote { repo, branch }
            if repo == "agent" && branch == "feat/login"),
        "{err:?}"
    );
    assert!(!fx.workspace_dir("pr-backend-12").exists());
    assert_eq!(git(&fx.cache_dir("backend"), &["branch", "--list"]), "");
}

#[test]
fn review_workspace_needs_a_choice_and_only_for_repos_of_the_project() {
    let fx = fixture_with_two_reviewable_branches();

    let nothing = review(&fx, &[]).unwrap_err();
    let unknown = review(&fx, &[("billing", "feat/login")]).unwrap_err();

    assert!(matches!(nothing, Error::NothingToReview), "{nothing:?}");
    assert!(
        matches!(&unknown, Error::RepoNotFound(repo) if repo == "billing"),
        "{unknown:?}"
    );
    assert!(!fx.workspace_dir("pr-backend-12").exists());
}

#[test]
fn a_branch_lives_in_one_workspace_at_a_time() {
    let fx = fixture_with_two_reviewable_branches();
    let own = checkout_workspace(&fx.registry(), PROJECT, "fix/prompt", None, false)
        .unwrap()
        .path;

    let reviewed = review(&fx, &[("agent", "fix/prompt")]).unwrap_err();
    review(&fx, &[("backend", "feat/login")]).unwrap();
    let checked_out =
        checkout_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap_err();

    assert!(
        matches!(&reviewed, Error::BranchInUse { repo, branch, path }
            if repo == "agent" && branch == "fix/prompt"
                && path.canonicalize().unwrap() == own.join("agent").canonicalize().unwrap()),
        "{reviewed:?}"
    );
    assert!(
        matches!(&checked_out, Error::BranchInUse { repo, branch, .. }
            if repo == "backend" && branch == "feat/login"),
        "{checked_out:?}"
    );
    assert!(!fx.workspace_dir("feat-login").exists());
}

#[test]
fn review_workspace_adds_the_worktrees_missing_from_an_existing_one() {
    let fx = fixture_with_two_reviewable_branches();
    let path = review(&fx, &[("backend", "feat/login")]).unwrap().path;
    fs::remove_dir_all(path.join("backend")).unwrap();
    fx.register(&fx.remote("billing"));

    let workspace = review(&fx, &[("backend", "feat/login")]).unwrap();

    assert_eq!(
        workspace.repos_with(CheckoutOutcome::Kept),
        ["frontend", "agent"]
    );
    assert_eq!(workspace.repos_with(CheckoutOutcome::Restored), ["backend"]);
    assert_eq!(workspace.repos_with(CheckoutOutcome::Created), ["billing"]);
    assert_eq!(
        git(&path.join("backend"), &["branch", "--show-current"]),
        "feat/login"
    );
    assert_eq!(
        git(&path.join("billing"), &["branch", "--show-current"]),
        "pr-backend-12"
    );
}

#[test]
fn a_reviewed_workspace_is_pulled_and_removed_like_any_other() {
    let fx = fixture_with_two_reviewable_branches();
    let path = review(&fx, &[("backend", "feat/login")]).unwrap().path;
    commit_on(&fx.remote_dir("backend"), "feat/login", "review comments");

    let report = pull_workspace(&fx.home(), &fx.project(), "pr-backend-12").unwrap();
    let outcomes: Vec<_> = report
        .iter()
        .map(|repo| *repo.result.as_ref().unwrap())
        .collect();
    assert_eq!(
        outcomes,
        [
            PullOutcome::NoRemoteBranch,
            PullOutcome::Updated,
            PullOutcome::NoRemoteBranch
        ]
    );

    commit_here(&path.join("backend"), "a fix of mine");
    let err = remove_workspace(&fx.registry(), PROJECT, "pr-backend-12", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceHasWork { reasons, .. }
            if reasons == &["backend: 1 unpushed commit on `feat/login`"]),
        "{err:?}"
    );
    remove_workspace(&fx.registry(), PROJECT, "pr-backend-12", true).unwrap();
    assert!(!path.exists());
    for repo in ["frontend", "backend", "agent"] {
        assert_eq!(git(&fx.cache_dir(repo), &["branch", "--list"]), "");
    }
}

#[test]
fn a_review_is_remembered_so_any_command_completes_it_with_the_same_branches() {
    let fx = fixture_with_two_reviewable_branches();
    let path = review(&fx, &[("backend", "feat/login"), ("agent", "fix/prompt")])
        .unwrap()
        .path;
    commit_here(&path.join("backend"), "a fix of mine");
    let mine = head(&path.join("backend"), "HEAD");
    fs::remove_dir_all(path.join("backend")).unwrap();
    fs::remove_dir_all(path.join("frontend")).unwrap();

    let listed = status(&fx.home(), &fx.project()).unwrap();
    let workspace =
        checkout_workspace(&fx.registry(), PROJECT, "pr-backend-12", None, false).unwrap();

    assert_eq!(
        listed.workspaces[0].branch.as_deref(),
        Some("pr-backend-12")
    );
    assert_eq!(
        workspace.repos_with(CheckoutOutcome::Restored),
        ["frontend", "backend"]
    );
    assert_eq!(workspace.repos_with(CheckoutOutcome::Kept), ["agent"]);
    assert_eq!(
        git(&path.join("backend"), &["branch", "--show-current"]),
        "feat/login"
    );
    assert_eq!(head(&path.join("backend"), "HEAD"), mine);
    assert_eq!(
        git(&path.join("frontend"), &["branch", "--show-current"]),
        "pr-backend-12"
    );
}

#[test]
fn a_workspace_reviews_what_it_was_made_for_and_nothing_else() {
    let fx = fixture_with_two_reviewable_branches();
    review(&fx, &[("backend", "feat/login")]).unwrap();
    create_workspace(&fx.registry(), PROJECT, "work", None, false).unwrap();
    let onto = |branch: &str, chosen: &[(&str, &str)]| {
        let chosen = chosen
            .iter()
            .map(|(repo, branch)| (repo.to_string(), branch.to_string()))
            .collect();
        review_workspace(&fx.registry(), PROJECT, branch, &chosen, None, false).unwrap_err()
    };

    let more = review(&fx, &[("backend", "feat/login"), ("agent", "fix/prompt")]).unwrap_err();
    let plain = onto("work", &[("agent", "fix/prompt")]);

    assert!(
        matches!(&more, Error::ReviewsOtherBranches { name, reviewed }
            if name == "pr-backend-12" && reviewed == &["backend=feat/login"]),
        "{more:?}"
    );
    assert!(
        matches!(&plain, Error::WorkspaceExists { name, .. } if name == "work"),
        "{plain:?}"
    );
    let agent = fx.workspace_dir("pr-backend-12").join("agent");
    assert_eq!(git(&agent, &["branch", "--show-current"]), "pr-backend-12");
    assert_eq!(fx.project().review("work"), None);
}

#[test]
fn a_review_that_fails_is_not_remembered() {
    let fx = fixture_with_two_reviewable_branches();

    review(&fx, &[("backend", "feat/nope")]).unwrap_err();

    assert!(fx.project().workspaces.is_empty());
}

#[test]
fn removing_a_review_deletes_its_branches_even_after_its_folder_is_gone() {
    let fx = fixture_with_two_reviewable_branches();
    let path = review(&fx, &[("backend", "feat/login"), ("agent", "fix/prompt")])
        .unwrap()
        .path;
    fs::remove_dir_all(&path).unwrap();

    remove_workspace(&fx.registry(), PROJECT, "pr-backend-12", false).unwrap();

    for repo in ["frontend", "backend", "agent"] {
        assert_eq!(git(&fx.cache_dir(repo), &["branch", "--list"]), "");
    }
    assert!(fx.project().workspaces.is_empty());
}

#[test]
fn remove_workspace_leaves_alone_a_branch_that_another_workspace_holds() {
    let fx = fixture_with_two_reviewable_branches();
    let path = review(&fx, &[("backend", "feat/login")]).unwrap().path;

    let err = remove_workspace(&fx.registry(), PROJECT, "feat/login", true).unwrap_err();

    assert!(
        matches!(&err, Error::WorkspaceNotFound(branch) if branch == "feat/login"),
        "{err:?}"
    );
    assert_eq!(
        git(&path.join("backend"), &["branch", "--show-current"]),
        "feat/login"
    );
}

#[test]
fn workspace_path_finds_an_existing_workspace_by_branch() {
    let fx = fixture_with_repos(&["frontend"]);
    let created = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;

    assert_eq!(
        workspace_path(&fx.home(), &fx.project(), "feat/login").unwrap(),
        created
    );
    let err = workspace_path(&fx.home(), &fx.project(), "feat/other").unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceNotFound(branch) if branch == "feat/other"),
        "{err:?}"
    );
}

#[test]
fn locate_workspace_recognises_any_directory_inside_a_workspace() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
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
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
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
        .map(|repo| (repo.key.as_str(), *repo.result.as_ref().unwrap()))
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
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
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
        .map(|repo| *repo.result.as_ref().unwrap())
        .collect();
    assert_eq!(outcomes, [PullOutcome::UpToDate, PullOutcome::Diverged]);
    assert_eq!(head(&path.join("backend"), "HEAD"), local_head);
}

#[test]
fn pull_workspace_skips_repos_without_a_worktree_or_a_branch() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
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
        keep_project_base,
    )
    .unwrap();

    let report = pull_workspace(&fx.home(), &fx.project(), "feat-login").unwrap();

    let outcomes: Vec<_> = report
        .iter()
        .map(|repo| *repo.result.as_ref().unwrap())
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
    let login = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    create_workspace(&fx.registry(), PROJECT, "fix/typo", None, false).unwrap();
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
    assert!(checkout.error.is_none());
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
        keep_project_base,
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
    register_repo(
        &fx.registry(),
        PROJECT,
        &repo,
        |_, _| true,
        keep_project_base,
    )
    .unwrap();

    let status = status(&fx.home(), &fx.project()).unwrap();

    assert!(status.repos[0].cached);
    assert!(status.repos[0].commit.is_none());
}

#[test]
fn status_reports_a_broken_worktree_without_hiding_the_others() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();
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
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;

    let name = remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

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
    create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();
}

#[test]
fn remove_workspace_refuses_to_lose_uncommitted_or_unpushed_work() {
    let fx = fixture_with_repos(&["frontend", "backend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    fs::write(path.join("frontend").join("draft.txt"), "wip").unwrap();
    commit_here(&path.join("backend"), "local work");
    commit_here(&path.join("backend"), "more local work");

    let err = remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap_err();

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
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    git(&path.join("frontend"), &["push", "--quiet"]);

    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

    assert!(!path.exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn remove_workspace_with_force_discards_local_work() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    commit_here(&path.join("frontend"), "local work");
    fs::write(path.join("frontend").join("draft.txt"), "wip").unwrap();
    fs::write(path.join("notes.md"), "mine").unwrap();

    let err = remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceHasWork { reasons, .. }
            if reasons.contains(&"other files in the workspace: notes.md".to_string())),
        "{err:?}"
    );
    remove_workspace(&fx.registry(), PROJECT, "feat/login", true).unwrap();

    assert!(!path.exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn remove_workspace_cleans_up_after_a_folder_deleted_by_hand() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    fs::remove_dir_all(&path).unwrap();

    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
    let err = remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceNotFound(branch) if branch == "feat/login"),
        "{err:?}"
    );
}

#[test]
fn remove_workspace_with_force_removes_a_broken_worktree() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(&fx.registry(), PROJECT, "feat/login", None, false)
        .unwrap()
        .path;
    fs::remove_dir_all(fx.cache_dir("frontend").join("worktrees")).unwrap();

    let err = remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap_err();
    assert!(
        matches!(&err, Error::WorkspaceHasWork { reasons, .. }
            if reasons == &["frontend: the worktree cannot be read"]),
        "{err:?}"
    );
    remove_workspace(&fx.registry(), PROJECT, "feat/login", true).unwrap();

    assert!(!path.exists());
    assert_eq!(git(&fx.cache_dir("frontend"), &["branch", "--list"]), "");
}

#[test]
fn a_workspace_keeps_its_own_location_until_it_is_removed() {
    let fx = fixture_with_repos(&["frontend"]);
    let placed = || fx.project().workspaces.get("feat-login").cloned();

    create_workspace(
        &fx.registry(),
        PROJECT,
        "feat/login",
        Some(&Location::Local),
        false,
    )
    .unwrap();
    assert_eq!(
        placed().and_then(|settings| settings.location),
        Some(Location::Local)
    );

    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();
    assert_eq!(placed(), None);
    assert!(fx.cache_dir("frontend").is_dir());
}

#[test]
fn removing_a_workspace_forgets_its_run_commands() {
    let fx = fixture_with_repos(&["frontend"]);
    create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();
    fx.registry()
        .update_project(PROJECT, |project| {
            project.set_workspace_run("feat-login", "frontend", Some("npm run dev"))
        })
        .unwrap();

    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

    assert!(fx.project().workspaces.is_empty());
}

#[test]
fn a_location_left_by_a_workspace_deleted_by_hand_is_forgotten_by_remove() {
    let fx = fixture_with_repos(&["frontend"]);
    let path = create_workspace(
        &fx.registry(),
        PROJECT,
        "feat/login",
        Some(&Location::Local),
        false,
    )
    .unwrap()
    .path;
    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();
    fx.registry()
        .update_project(PROJECT, |project| {
            project.set_workspace_location("feat-login", Some(Location::Local))
        })
        .unwrap();
    assert!(!path.exists());

    remove_workspace(&fx.registry(), PROJECT, "feat/login", false).unwrap();

    assert!(fx.project().workspaces.is_empty());
}
