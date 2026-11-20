//! Runs the `console` examples of the pages in `docs/src` against the real `octok`, so an
//! example that no longer matches what `octok` prints fails the build.
//!
//! `BOOK_PAGE=<topic>` runs one page, and `BOOK_UPDATE=1` rewrites the expected output of the
//! examples from what the commands print.

mod page;
mod remotes;
mod runner;
mod sandbox;
mod shell;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use tempfile::TempDir;

use runner::{Mode, run_page, workspace_root};
use sandbox::Sandbox;

#[test]
fn the_examples_of_the_book_match_octok() {
    let mode = if env::var_os("BOOK_UPDATE").is_some() {
        Mode::Update
    } else {
        Mode::Check
    };
    let pages = book_pages(env::var("BOOK_PAGE").ok().as_deref());

    let failures: Vec<String> = thread::scope(|scope| {
        let runs: Vec<_> = pages
            .iter()
            .map(|page| scope.spawn(move || run_page(page, mode)))
            .collect();
        runs.into_iter()
            .filter_map(|run| run.join().unwrap().err())
            .collect()
    });

    assert!(
        failures.is_empty(),
        "{}\n\nrewrite the output of a page with `BOOK_PAGE=<topic> BOOK_UPDATE=1 cargo test -p octotask --test book`",
        failures.join("\n\n")
    );
}

#[test]
fn a_matching_example_passes() {
    assert_eq!(run_page(&fixture_page("passing.md"), Mode::Check), Ok(()));
}

#[test]
fn a_differing_example_shows_the_page_the_command_and_both_outputs() {
    let failure = run_page(&fixture_page("failing.md"), Mode::Check).unwrap_err();

    assert_eq!(
        failure,
        "crates/octotask-cli/tests/book/pages/failing.md:6: $ echo hello\n\
         expected:\n  goodbye\n\
         actual:\n  hello"
    );
}

#[test]
fn state_carries_from_one_block_to_the_next() {
    assert_eq!(run_page(&fixture_page("state.md"), Mode::Check), Ok(()));
}

#[test]
fn the_wildcard_matches_any_text_within_one_line() {
    assert_eq!(run_page(&fixture_page("wildcard.md"), Mode::Check), Ok(()));

    let failure = run_page(&fixture_page("wildcard-one-line.md"), Mode::Check).unwrap_err();
    assert!(failure.contains("$ printf 'a\\nb\\n'"), "{failure}");
}

#[test]
fn update_mode_rewrites_the_output_and_keeps_the_rest_of_the_page() {
    let dir = TempDir::new().unwrap();
    let page = dir.path().join("outdated.md");
    fs::copy(fixture_page("outdated.md"), &page).unwrap();

    assert_eq!(run_page(&page, Mode::Update), Ok(()));

    assert_eq!(
        fs::read_to_string(&page).unwrap(),
        fs::read_to_string(fixture_page("outdated-updated.md")).unwrap()
    );
    assert_eq!(run_page(&page, Mode::Check), Ok(()));
}

#[test]
fn a_page_without_console_blocks_is_skipped() {
    let dir = TempDir::new().unwrap();
    let page = dir.path().join("prose.md");
    fs::write(&page, "# Prose\n\n```sh\n$ false\nnot checked\n```\n").unwrap();

    assert_eq!(run_page(&page, Mode::Check), Ok(()));
}

#[test]
fn the_sandbox_has_its_own_home_and_shows_it_as_a_reader_would_see_it() {
    assert_eq!(run_page(&fixture_page("sandbox.md"), Mode::Check), Ok(()));
}

#[test]
fn the_fixture_remotes_answer_at_their_github_urls_with_stable_commits_and_dates() {
    assert_eq!(run_page(&fixture_page("remotes.md"), Mode::Check), Ok(()));
}

#[test]
fn the_sandbox_has_no_network() {
    assert_eq!(
        run_page(&fixture_page("no-network.md"), Mode::Check),
        Ok(())
    );
}

#[test]
fn octok_runs_in_the_sandbox() {
    assert_eq!(run_page(&fixture_page("octok.md"), Mode::Check), Ok(()));
}

/// Checked on the raw output: the page shows the local path of a remote as its GitHub url.
#[test]
fn octok_prints_the_github_urls_and_never_the_local_path_of_a_remote() {
    let sandbox = Sandbox::new();
    let mut shell = sandbox.shell();
    let local_path = "/remotes/acme/";
    let url = shell
        .run("git ls-remote --get-url git@github.com:acme/shop-frontend.git")
        .unwrap();
    assert!(url.contains(local_path), "{url}");
    let commands = [
        "octok project init shop",
        "octok repo add git@github.com:acme/shop-frontend.git --alias frontend",
        "octok repo add git@github.com:acme/shop-backend.git --alias backend",
        "octok repo add git@github.com:acme/shop-agent.git --alias agent",
        "octok repo ls",
        "octok repo ls --json",
        "octok repo set backend --url git@github.com:acme/shop-backend.git",
        "octok fetch",
        "octok checkout feat/login",
        "octok pull feat/login",
        "octok pr review --pr backend=12 --branch agent=fix/prompt",
        "octok status",
        "octok status --json",
        "octok rm feat/login --json",
    ];

    for command in commands {
        let output = shell.run(command).unwrap();
        assert!(!output.contains(local_path), "{command}: {output}");
    }
}

/// The pages of the book, in a stable order: all of them, or the one of `topic`.
fn book_pages(topic: Option<&str>) -> Vec<PathBuf> {
    let root = workspace_root().join("docs/src");
    assert!(root.is_dir(), "no book at {}", root.display());
    let mut pages = markdown_files(&root);
    pages.sort();
    let Some(topic) = topic else {
        return pages;
    };
    let wanted = root.join(format!("{}.md", topic_of(topic)));
    assert!(pages.contains(&wanted), "no page `{topic}` in docs/src");
    vec![wanted]
}

/// `commands/project`, given as the topic, the file or its path from the repository root.
fn topic_of(page: &str) -> &str {
    let page = page.strip_prefix("docs/src/").unwrap_or(page);
    page.strip_suffix(".md").unwrap_or(page)
}

fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(markdown_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    files
}

fn fixture_page(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/book/pages")
        .join(name)
}
