//! Embeds the pages of the book that `docs/src/SUMMARY.md` lists, in its order, for
//! `octok docs`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let book = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/src");
    let book = book
        .canonicalize()
        .unwrap_or_else(|error| panic!("{}: {error}", book.display()));
    println!("cargo::rerun-if-changed={}", book.display());

    let summary = read(&book.join("SUMMARY.md"));
    let table: String = chapters(&summary)
        .map(|(title, path)| page_entry(&book, title, path))
        .collect();

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let pages = out.join("pages.rs");
    fs::write(&pages, format!("&[\n{table}]\n"))
        .unwrap_or_else(|error| panic!("{}: {error}", pages.display()));
}

/// The links of `SUMMARY.md`, in order, as `(title, path)`: `[Title](path.md)`, as a list
/// item or alone. The part titles and the separators are not pages.
fn chapters(summary: &str) -> impl Iterator<Item = (&str, &str)> {
    summary.lines().filter_map(|line| {
        let line = line.trim_start().trim_start_matches(['-', '*']).trim();
        line.strip_prefix('[')?.strip_suffix(')')?.split_once("](")
    })
}

/// One `Page` of the table, or a failed build when the page has no file.
fn page_entry(book: &Path, title: &str, path: &str) -> String {
    let file = book.join(path);
    let topic = path.strip_suffix(".md");
    let (Some(topic), true) = (topic, file.is_file()) else {
        panic!(
            "docs/src/SUMMARY.md lists the page `{title}` at `{path}`, which is not a Markdown file of docs/src"
        );
    };
    format!(
        "    Page {{ topic: {topic:?}, title: {title:?}, markdown: include_str!({:?}) }},\n",
        file.display().to_string()
    )
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}
