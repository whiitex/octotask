//! Runs the examples of one page in a sandbox of its own, and checks or rewrites their output.

use std::fs;
use std::path::{Path, PathBuf};

use crate::page::{Example, Page};
use crate::sandbox::Sandbox;

#[derive(Clone, Copy)]
pub enum Mode {
    /// Fails on the first output that differs from the page.
    Check,
    /// Writes the real output into the page.
    Update,
}

/// Runs the examples of `path` in order, in one shell. A page without examples is skipped.
pub fn run_page(path: &Path, mode: Mode) -> Result<(), String> {
    let page = Page::read(path);
    if page.examples.is_empty() {
        return Ok(());
    }
    let sandbox = Sandbox::new();
    let mut shell = sandbox.shell();
    let mut outputs = Vec::new();
    for example in &page.examples {
        let raw = shell
            .run(&example.command)
            .map_err(|error| format!("{}\n{error}", location(&page, example)))?;
        let actual = output_lines(&sandbox.redact(&raw));
        match mode {
            Mode::Check if !example.matches(&actual) => {
                return Err(mismatch(&page, example, &actual));
            }
            Mode::Check => {}
            Mode::Update => outputs.push(example.updated(actual)),
        }
    }
    if let Mode::Update = mode {
        fs::write(path, page.rewritten(&outputs)).unwrap();
    }
    Ok(())
}

/// The root of the repository, where `book/` is.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The lines of `output` without trailing spaces and trailing blank lines, as a page has them.
fn output_lines(output: &str) -> Vec<String> {
    let mut lines: Vec<String> = output
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn mismatch(page: &Page, example: &Example, actual: &[String]) -> String {
    format!(
        "{}\nexpected:\n{}\nactual:\n{}",
        location(page, example),
        indented(&example.expected),
        indented(actual)
    )
}

/// `docs/src/commands/project.md:12: $ octok repo ls`
fn location(page: &Page, example: &Example) -> String {
    let path = page.path.canonicalize().unwrap_or(page.path.clone());
    let path = path.strip_prefix(workspace_root()).unwrap_or(&path);
    format!(
        "{}:{}: $ {}",
        path.display(),
        example.line_number,
        example.command
    )
}

fn indented(lines: &[String]) -> String {
    if lines.is_empty() {
        return "  (nothing)".to_string();
    }
    lines
        .iter()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
