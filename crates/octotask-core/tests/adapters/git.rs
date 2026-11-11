use octotask_core::{Error, Git};
use tempfile::TempDir;

use super::{canonical, commit_empty, init_repo};

#[test]
fn run_returns_stdout_from_the_given_directory() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());

    let toplevel = Git::new(dir.path())
        .run(&["rev-parse", "--show-toplevel"])
        .unwrap();

    assert_eq!(toplevel.trim_end(), canonical(dir.path()));
}

#[test]
fn check_maps_exit_0_to_true_and_exit_1_to_false() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());
    let git = Git::new(dir.path());
    let head_exists = ["rev-parse", "--verify", "--quiet", "HEAD"];

    assert!(!git.check(&head_exists).unwrap());
    commit_empty(dir.path());
    assert!(git.check(&head_exists).unwrap());
}

#[test]
fn check_reports_other_exit_codes_as_errors() {
    let dir = TempDir::new().unwrap();

    let err = Git::new(dir.path())
        .check(&["rev-parse", "--verify", "--quiet", "HEAD"])
        .unwrap_err();

    assert!(
        matches!(&err, Error::CommandFailed { status, .. } if status.code() == Some(128)),
        "{err:?}"
    );
}

#[test]
fn bare_repo_with_origin_tracks_remote_branches_without_local_ones() {
    let remote = TempDir::new().unwrap();
    init_repo(remote.path());
    commit_empty(remote.path());
    let bare = TempDir::new().unwrap();
    let git = Git::new(bare.path());
    let remote_branch = Git::new(remote.path()).current_branch().unwrap().unwrap();

    git.init_bare().unwrap();
    git.add_origin(&remote.path().display().to_string())
        .unwrap();
    git.fetch_origin().unwrap();

    assert!(git.origin_branch_exists(&remote_branch).unwrap());
    assert!(!git.branch_exists(&remote_branch).unwrap());
    assert!(!git.origin_branch_exists("missing").unwrap());
}

#[test]
fn worktree_is_added_on_a_new_branch_and_removed_with_it() {
    let repo = TempDir::new().unwrap();
    init_repo(repo.path());
    commit_empty(repo.path());
    let git = Git::new(repo.path());
    let parent = TempDir::new().unwrap();
    let worktree = parent.path().join("feature");

    git.add_worktree(&worktree, "feat/login", "HEAD").unwrap();

    let checkout = Git::new(&worktree);
    assert_eq!(
        checkout.current_branch().unwrap().as_deref(),
        Some("feat/login")
    );
    checkout.run(&["checkout", "--quiet", "--detach"]).unwrap();
    assert_eq!(checkout.current_branch().unwrap(), None);

    git.remove_worktree(&worktree).unwrap();
    git.delete_branch("feat/login").unwrap();
    assert!(!worktree.exists());
    assert!(!git.branch_exists("feat/login").unwrap());
}

#[test]
fn last_commit_reads_hash_subject_and_dates() {
    let repo = TempDir::new().unwrap();
    init_repo(repo.path());
    commit_empty(repo.path());
    let git = Git::new(repo.path());

    let commit = git.last_commit("HEAD").unwrap();

    let full_hash = git.run(&["rev-parse", "HEAD"]).unwrap();
    assert!(full_hash.starts_with(&commit.sha) && commit.sha.len() >= 7);
    assert_eq!(commit.subject, "init");
    assert!(commit.date.contains('T'), "{}", commit.date);
    assert!(commit.age.ends_with("ago"), "{}", commit.age);
}

#[test]
fn fast_forward_moves_a_branch_only_along_its_own_history() {
    let repo = TempDir::new().unwrap();
    init_repo(repo.path());
    commit_empty(repo.path());
    let git = Git::new(repo.path());
    git.run(&["branch", "old"]).unwrap();
    commit_empty(repo.path());
    let newest = git.run(&["rev-parse", "HEAD"]).unwrap();
    let newest = newest.trim_end();
    git.run(&["checkout", "--quiet", "old"]).unwrap();

    assert!(git.is_ancestor("old", newest).unwrap());
    assert!(!git.is_ancestor(newest, "old").unwrap());
    git.fast_forward(newest).unwrap();

    assert_eq!(git.run(&["rev-parse", "HEAD"]).unwrap().trim_end(), newest);
}

#[test]
fn fetch_showing_progress_leaves_stderr_to_git() {
    let remote = TempDir::new().unwrap();
    init_repo(remote.path());
    commit_empty(remote.path());
    let remote_branch = Git::new(remote.path()).current_branch().unwrap().unwrap();
    let bare = TempDir::new().unwrap();
    let git = Git::new(bare.path());
    git.init_bare().unwrap();
    git.add_origin(&remote.path().display().to_string())
        .unwrap();

    git.fetch_origin_showing_progress().unwrap();
    assert!(git.origin_branch_exists(&remote_branch).unwrap());

    git.set_origin_url("/nonexistent/repo.git").unwrap();
    let err = git.fetch_origin_showing_progress().unwrap_err();
    // git wrote its message straight to the terminal, so there is nothing to repeat.
    assert!(
        matches!(&err, Error::CommandFailed { stderr, .. } if stderr.is_empty()),
        "{err:?}"
    );
}

#[test]
fn has_changes_sees_untracked_files() {
    let repo = TempDir::new().unwrap();
    init_repo(repo.path());
    commit_empty(repo.path());
    let git = Git::new(repo.path());

    assert!(!git.has_changes().unwrap());
    std::fs::write(repo.path().join("draft.txt"), "wip").unwrap();
    assert!(git.has_changes().unwrap());
}

#[test]
fn unpushed_commits_counts_what_no_remote_branch_contains() {
    let remote = TempDir::new().unwrap();
    init_repo(remote.path());
    commit_empty(remote.path());
    let bare = TempDir::new().unwrap();
    let git = Git::new(bare.path());
    git.init_bare().unwrap();
    git.add_origin(&remote.path().display().to_string())
        .unwrap();
    git.fetch_origin().unwrap();
    let remote_branch = Git::new(remote.path()).current_branch().unwrap().unwrap();
    let parent = TempDir::new().unwrap();
    let worktree = parent.path().join("feature");
    git.add_worktree(
        &worktree,
        "feat/login",
        &format!("refs/remotes/origin/{remote_branch}"),
    )
    .unwrap();

    assert_eq!(git.unpushed_commits("refs/heads/feat/login").unwrap(), 0);
    commit_empty(&worktree);
    commit_empty(&worktree);
    assert_eq!(git.unpushed_commits("refs/heads/feat/login").unwrap(), 2);
}

#[test]
fn a_worktree_deleted_by_hand_is_pruned_and_its_branch_checked_out_again() {
    let repo = TempDir::new().unwrap();
    init_repo(repo.path());
    commit_empty(repo.path());
    let git = Git::new(repo.path());
    let parent = TempDir::new().unwrap();
    let worktree = parent.path().join("feature");
    git.add_worktree(&worktree, "feat/login", "HEAD").unwrap();
    std::fs::remove_dir_all(&worktree).unwrap();

    git.prune_worktrees().unwrap();
    git.checkout_worktree(&worktree, "feat/login").unwrap();

    assert_eq!(
        Git::new(&worktree).current_branch().unwrap().as_deref(),
        Some("feat/login")
    );
}
