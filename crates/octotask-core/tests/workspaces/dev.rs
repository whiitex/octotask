use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use octotask_core::{
    Error, PullOutcome, create_workspace, dev_changes, pull_workspace, remove_project,
    remove_workspace, save_dev_changes, unregister_repo,
};

use super::{Fixture, PROJECT, commit_here, git};

const MAIN_PY: &str = "\
import os


def create_app():
    setup_observability(app)
    return app


if __name__ == \"__main__\":
    load_env()
    run(host=\"127.0.0.1\", port=8000)
";

/// A project with the repo `backend`, whose remote tracks `main.py` and `my config.yaml`
/// and ignores `.env`, and the workspace `feat-login`.
fn backend() -> Fixture {
    let fx = Fixture::new();
    let url = fx.remote("backend");
    commit_files(
        &fx.remote_dir("backend"),
        &[
            ("main.py", MAIN_PY),
            ("my config.yaml", "db: Agent\nmode: prod\n"),
            (".gitignore", ".env\n"),
        ],
    );
    fx.register(&url);
    create_workspace(&fx.registry(), PROJECT, "feat/login", None, false).unwrap();
    fx
}

/// Like `backend`, with the dev changes of `feat-login` saved.
fn backend_with_dev_changes() -> Fixture {
    let fx = backend();
    make_dev_changes(&checkout(&fx, "feat-login"));
    save(&fx, &[".env"]);
    fx
}

fn commit_files(repo: &Path, files: &[(&str, &str)]) {
    for (path, content) in files {
        fs::write(repo.join(path), content).unwrap();
    }
    git(repo, &["add", "--all"]);
    commit_here(repo, "change files");
}

fn checkout(fx: &Fixture, workspace: &str) -> PathBuf {
    fx.workspace_dir(workspace).join("backend")
}

fn replace(file: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(file).unwrap();
    assert!(text.contains(from), "{from:?} is not in {text}");
    fs::write(file, text.replace(from, to)).unwrap();
}

/// A dev port and a dev database in tracked files, a secret that git ignores and a file
/// that git has never seen.
fn make_dev_changes(checkout: &Path) {
    replace(&checkout.join("main.py"), "port=8000", "port=9005");
    replace(
        &checkout.join("my config.yaml"),
        "db: Agent",
        "db: DeepTree",
    );
    fs::write(checkout.join(".env"), "TOKEN=secret\n").unwrap();
    fs::write(checkout.join("notes.txt"), "dev notes\n").unwrap();
}

fn save(fx: &Fixture, files: &[&str]) -> octotask_core::DevChanges {
    let mut saved = save_dev_changes(&fx.home(), &fx.project(), "feat-login", &[], files).unwrap();
    assert_eq!(saved.len(), 1);
    saved.remove(0)
}

fn read(checkout: &Path, file: &str) -> String {
    fs::read_to_string(checkout.join(file)).unwrap()
}

fn status(checkout: &Path) -> String {
    git(checkout, &["status", "--porcelain"])
}

fn new_workspace(fx: &Fixture, branch: &str) -> PathBuf {
    create_workspace(&fx.registry(), PROJECT, branch, None, false).unwrap();
    checkout(fx, &branch.replace('/', "-"))
}

#[test]
fn save_takes_the_uncommitted_changes_and_the_files_git_does_not_track() {
    let fx = backend();
    let checkout = checkout(&fx, "feat-login");
    make_dev_changes(&checkout);

    let saved = save(&fx, &[".env"]);

    assert_eq!(saved.repo, "backend");
    assert_eq!(saved.changed, ["main.py", "my config.yaml"]);
    assert_eq!(saved.new, [".env", "notes.txt"]);
    assert!(
        saved
            .diff
            .contains("-    run(host=\"127.0.0.1\", port=8000)")
    );
    assert!(
        saved
            .diff
            .contains("+    run(host=\"127.0.0.1\", port=9005)")
    );
    assert!(
        saved.diff.contains("+++ b/my config.yaml"),
        "{}",
        saved.diff
    );
    assert!(!saved.diff.contains("secret"), "{}", saved.diff);
    assert_eq!(status(&checkout), "", "git no longer sees the dev changes");
    assert!(read(&checkout, "main.py").contains("port=9005"));
}

#[test]
fn an_ignored_file_is_saved_only_when_it_is_named_and_must_exist() {
    let fx = backend();
    make_dev_changes(&checkout(&fx, "feat-login"));

    assert_eq!(save(&fx, &[]).new, ["notes.txt"]);

    let err =
        save_dev_changes(&fx.home(), &fx.project(), "feat-login", &[], &["nope.env"]).unwrap_err();
    assert!(
        matches!(&err, Error::DevFileNotFound(file) if file == "nope.env"),
        "{err:?}"
    );
    for outside in ["/etc/hosts", "../other/.env", ""] {
        let err =
            save_dev_changes(&fx.home(), &fx.project(), "feat-login", &[], &[outside]).unwrap_err();
        assert!(
            matches!(err, Error::InvalidName { .. }),
            "{outside:?}: {err:?}"
        );
    }
}

#[test]
fn a_new_workspace_comes_with_the_dev_changes_and_git_does_not_see_them() {
    let fx = backend_with_dev_changes();

    let other = new_workspace(&fx, "feat/other");

    assert!(read(&other, "main.py").contains("port=9005"));
    assert_eq!(read(&other, "my config.yaml"), "db: DeepTree\nmode: prod\n");
    assert_eq!(read(&other, ".env"), "TOKEN=secret\n");
    assert_eq!(read(&other, "notes.txt"), "dev notes\n");
    assert_eq!(status(&other), "");
    assert_eq!(git(&other, &["diff", "HEAD"]), "");
}

#[test]
fn dev_changes_stay_out_of_a_commit_that_changes_the_same_file() {
    let fx = backend_with_dev_changes();
    let other = new_workspace(&fx, "feat/other");

    replace(
        &other.join("main.py"),
        "import os\n",
        "import os\nimport sys\n",
    );
    fs::write(other.join("feature.py"), "print('feature')\n").unwrap();
    assert_eq!(status(&other), "M main.py\n?? feature.py");
    git(&other, &["add", "--all"]);
    commit_here(&other, "real work");

    let committed = git(&other, &["show", "HEAD:main.py"]);
    assert!(committed.contains("import sys"), "{committed}");
    assert!(committed.contains("port=8000"), "{committed}");
    let files = git(&other, &["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(files, "feature.py\nmain.py");
    assert!(read(&other, "main.py").contains("port=9005"));
    assert_eq!(status(&other), "");
}

#[test]
fn a_change_to_the_lines_of_the_dev_changes_cannot_be_staged() {
    let fx = backend_with_dev_changes();
    let other = new_workspace(&fx, "feat/other");
    replace(&other.join("main.py"), "port=9005", "port=9999");

    let add = Command::new("git")
        .arg("-C")
        .arg(&other)
        .args(["add", "main.py"])
        .output()
        .unwrap();

    assert!(!add.status.success());
    let stderr = String::from_utf8_lossy(&add.stderr);
    assert!(stderr.contains("octok dev save"), "{stderr}");
}

#[test]
fn saving_again_follows_the_checkout() {
    let fx = backend_with_dev_changes();
    let login = checkout(&fx, "feat-login");

    replace(&login.join("main.py"), "port=9005", "port=9006");
    fs::remove_file(login.join("notes.txt")).unwrap();
    let saved = save(&fx, &[]);
    assert_eq!(saved.changed, ["main.py", "my config.yaml"]);
    assert_eq!(
        saved.new,
        [".env"],
        "a file saved before stays while it is there"
    );
    let other = new_workspace(&fx, "feat/other");
    assert!(read(&other, "main.py").contains("port=9006"));
    assert!(!other.join("notes.txt").exists());

    replace(&login.join("main.py"), "port=9006", "port=8000");
    replace(&login.join("my config.yaml"), "db: DeepTree", "db: Agent");
    fs::remove_file(login.join(".env")).unwrap();
    let saved = save(&fx, &[]);
    assert!(
        saved.changed.is_empty() && saved.new.is_empty(),
        "{saved:?}"
    );
    let third = new_workspace(&fx, "feat/third");
    assert_eq!(read(&third, "main.py"), MAIN_PY);
    assert!(!third.join(".env").exists());
}

#[test]
fn dev_changes_lists_what_is_saved_for_each_repo() {
    let fx = backend_with_dev_changes();
    fx.register(&fx.remote("frontend"));

    let all = dev_changes(&fx.home(), &fx.project(), &[]).unwrap();
    let repos: Vec<&str> = all.iter().map(|changes| changes.repo.as_str()).collect();
    assert_eq!(repos, ["backend", "frontend"]);
    assert_eq!(all[0].changed, ["main.py", "my config.yaml"]);
    assert!(all[1].changed.is_empty() && all[1].new.is_empty());

    let one = dev_changes(&fx.home(), &fx.project(), &["frontend"]).unwrap();
    assert_eq!(one.len(), 1);
    let err = dev_changes(&fx.home(), &fx.project(), &["nope"]).unwrap_err();
    assert!(matches!(err, Error::RepoNotFound(_)), "{err:?}");
}

#[test]
fn a_workspace_is_refused_when_the_dev_changes_no_longer_apply_to_the_base() {
    let fx = backend_with_dev_changes();
    let upstream = MAIN_PY.replace("port=8000", "port=8080, workers=4");
    commit_files(&fx.remote_dir("backend"), &[("main.py", &upstream)]);

    let err = create_workspace(&fx.registry(), PROJECT, "feat/other", None, false).unwrap_err();

    assert!(
        matches!(&err, Error::StaleDevChanges { repo, files }
            if repo == "backend" && files == &["main.py"]),
        "{err:?}"
    );
    assert!(err.to_string().contains("--allow-stale-dev"), "{err}");
    assert!(!fx.workspace_dir("feat-other").exists());
    assert_eq!(
        git(
            &fx.cache_dir("backend"),
            &["branch", "--list", "feat/other"]
        ),
        ""
    );

    create_workspace(&fx.registry(), PROJECT, "feat/other", None, true).unwrap();
    let other = checkout(&fx, "feat-other");
    assert_eq!(read(&other, "main.py"), upstream, "left as it is in git");
    assert_eq!(read(&other, "my config.yaml"), "db: DeepTree\nmode: prod\n");
    assert_eq!(read(&other, ".env"), "TOKEN=secret\n");
    assert_eq!(status(&other), "");
}

#[test]
fn pull_keeps_the_dev_changes_of_a_file_that_changes_on_the_remote() {
    let fx = backend_with_dev_changes();
    let other = new_workspace(&fx, "feat/other");
    let remote = fx.remote_dir("backend");
    git(&remote, &["checkout", "--quiet", "-b", "feat/other"]);
    commit_files(
        &remote,
        &[("main.py", &MAIN_PY.replace("import os\n", "import sys\n"))],
    );

    let report = pull_workspace(&fx.home(), &fx.project(), "feat-other").unwrap();

    assert!(
        matches!(report[0].1, Ok(PullOutcome::Updated)),
        "{report:?}"
    );
    let pulled = read(&other, "main.py");
    assert!(
        pulled.contains("import sys") && pulled.contains("port=9005"),
        "{pulled}"
    );
    assert_eq!(status(&other), "");
}

#[test]
fn a_workspace_with_only_dev_changes_is_removed_without_force() {
    let fx = backend_with_dev_changes();
    new_workspace(&fx, "feat/other");

    remove_workspace(&fx.registry(), PROJECT, "feat/other", false).unwrap();

    assert!(!fx.workspace_dir("feat-other").exists());
}

#[test]
fn the_saved_files_are_kept_by_kind() {
    let fx = backend_with_dev_changes();
    let saved = fx.home_dir().join("dev").join(PROJECT).join("backend");
    let names = |dir: &Path| {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };

    assert_eq!(names(&saved), ["base", "dev", "filter.sh"]);
    assert_eq!(names(&saved.join("base")), ["main.py", "my config.yaml"]);
    assert_eq!(names(&saved.join("dev")), ["changed", "new"]);
    assert_eq!(
        names(&saved.join("dev/changed")),
        ["main.py", "my config.yaml"]
    );
    assert_eq!(names(&saved.join("dev/new")), [".env", "notes.txt"]);
    assert_eq!(read(&saved.join("base"), "main.py"), MAIN_PY);
    assert!(read(&saved.join("dev/changed"), "main.py").contains("port=9005"));
}

#[test]
fn the_saved_dev_changes_go_away_with_the_repo_and_with_the_project() {
    let saved = |fx: &Fixture| fx.home_dir().join("dev").join(PROJECT).join("backend");

    let fx = backend_with_dev_changes();
    assert!(saved(&fx).is_dir());
    remove_workspace(&fx.registry(), PROJECT, "feat/login", true).unwrap();
    unregister_repo(&fx.registry(), PROJECT, "backend").unwrap();
    assert!(!saved(&fx).exists());

    let fx = backend_with_dev_changes();
    remove_project(&fx.registry(), PROJECT, true).unwrap();
    assert!(!fx.home_dir().join("dev").join(PROJECT).exists());
}
