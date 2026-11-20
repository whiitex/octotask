//! Tests for `src/docs.rs`: `octok docs`, and the conventions of the pages of `docs/src`
//! that it prints, so a page that the binary, the site and `llms.txt` would read
//! differently fails here.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use serde_json::{Value, json};

/// The page that `octok` renders from its own command line instead of reading a file.
const COMMAND_REFERENCE: &str = "reference/commands";

/// A page as `SUMMARY.md` lists it.
struct Entry {
    title: String,
    topic: String,
}

fn book_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/src")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn page_file(topic: &str) -> PathBuf {
    book_dir().join(format!("{topic}.md"))
}

/// The links of `SUMMARY.md`, in order: `[Title](path.md)`, as a list item or alone.
fn summary_entries() -> Vec<Entry> {
    read(&book_dir().join("SUMMARY.md"))
        .lines()
        .filter_map(|line| {
            let line = line.trim_start().trim_start_matches(['-', '*']).trim();
            let (title, path) = line
                .strip_prefix('[')?
                .strip_suffix(')')?
                .split_once("](")?;
            let topic = path.strip_suffix(".md")?;
            Some(Entry {
                title: title.to_string(),
                topic: topic.to_string(),
            })
        })
        .collect()
}

/// `octok`, without the home and the project that every other command needs.
fn octok(args: &[&str]) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("octok"));
    cmd.env_remove("OCTOTASK_HOME")
        .env_remove("HOME")
        .args(args);
    cmd
}

fn stdout(args: &[&str]) -> String {
    let out = octok(args).output().unwrap();
    assert!(
        out.status.success(),
        "octok {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn document(args: &[&str]) -> Value {
    serde_json::from_str(&stdout(args)).unwrap()
}

/// The line under the title of a page.
fn summary(markdown: &str) -> &str {
    markdown.lines().nth(2).unwrap_or_default()
}

/// The section of the command reference about `command`, from its heading to the next one.
fn reference_section<'a>(reference: &'a str, command: &str) -> &'a str {
    let heading = format!("\n## `{command}`\n");
    let start = reference
        .find(&heading)
        .unwrap_or_else(|| panic!("the command reference has no section for `{command}`"));
    let rest = &reference[start + heading.len()..];
    &rest[..rest.find("\n## ").unwrap_or(rest.len())]
}

/// The words of the code spans that start the items of a section: `-p`, `--project`,
/// `<PROJECT>` for ``- `-p, --project <PROJECT>`: ...``.
fn documented(section: &str) -> BTreeSet<String> {
    section
        .lines()
        .filter_map(|line| line.strip_prefix("- `")?.split_once('`'))
        .flat_map(|(code, _)| code.split([' ', ',']))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

/// The commands under a command that its `--help` lists, without `help`.
fn subcommands(help: &str) -> Vec<String> {
    help_section(help, "Commands:")
        .iter()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| *name != "help")
        .map(str::to_string)
        .collect()
}

/// What `--help` lists under `Arguments:` and `Options:`: `<NAME>`, `[REPOS]...`, `--long`.
fn arguments_and_options(help: &str) -> Vec<String> {
    let arguments = help_section(help, "Arguments:")
        .into_iter()
        .filter_map(|line| line.split_whitespace().next());
    let options = help_section(help, "Options:")
        .into_iter()
        .filter(|line| line.starts_with('-'))
        .flat_map(|line| line.split([' ', ',', '=']))
        .filter(|word| word.starts_with("--") && *word != "--help");
    arguments.chain(options).map(str::to_string).collect()
}

/// The lines of a section of `--help`, trimmed, without the lines of help text.
fn help_section<'a>(help: &'a str, title: &str) -> Vec<&'a str> {
    help.lines()
        .skip_while(|line| *line != title)
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter(|line| line.starts_with("  ") && !line.starts_with("          "))
        .map(str::trim)
        .collect()
}

/// Every Markdown file under `docs/src`, by topic, `SUMMARY.md` aside.
fn page_topics() -> BTreeSet<String> {
    fn walk(dir: &Path, topics: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, topics);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                let relative = path.strip_prefix(book_dir()).unwrap().with_extension("");
                topics.insert(relative.to_str().unwrap().to_string());
            }
        }
    }
    let mut topics = BTreeSet::new();
    walk(&book_dir(), &mut topics);
    topics.remove("SUMMARY");
    topics
}

/// The lines of a page outside its code blocks. A block opened by N backticks ends at a
/// line of at least N of them, so a ```` block can show a ``` one.
fn prose(markdown: &str) -> Vec<&str> {
    let mut open_fence: Option<usize> = None;
    markdown
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            let ticks = leading_backticks(line);
            match open_fence {
                Some(opened) => {
                    if ticks >= opened && line[ticks..].trim().is_empty() {
                        open_fence = None;
                    }
                    false
                }
                None if ticks >= 3 => {
                    open_fence = Some(ticks);
                    false
                }
                None => true,
            }
        })
        .collect()
}

fn leading_backticks(text: &str) -> usize {
    text.len() - text.trim_start_matches('`').len()
}

/// `line` without its code spans: what Markdown reads as text and HTML.
fn outside_code_spans(line: &str) -> String {
    let mut text = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        text.push_str(&rest[..start]);
        let ticks = leading_backticks(&rest[start..]);
        let inside = &rest[start + ticks..];
        match closing_backticks(inside, ticks) {
            Some(end) => rest = &inside[end + ticks..],
            None => {
                text.push_str(&rest[start..start + ticks]);
                rest = inside;
            }
        }
    }
    text.push_str(rest);
    text
}

/// Where the run of exactly `ticks` backticks that closes a code span starts in `text`.
fn closing_backticks(text: &str, ticks: usize) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = text[from..].find('`') {
        let start = from + found;
        let run = leading_backticks(&text[start..]);
        if run == ticks {
            return Some(start);
        }
        from = start + run;
    }
    None
}

/// Whether `text` has a `<` that Markdown takes as the start of HTML or of an autolink, such
/// as `<branch>`, `</b>`, `<!-- -->` or `<https://...>`: the site would hide it or render it.
fn has_html(text: &str) -> bool {
    text.match_indices('<').any(|(start, _)| {
        let escaped = text[..start].ends_with('\\');
        let next = text[start + 1..].chars().next();
        !escaped && next.is_some_and(|next| next.is_ascii_alphabetic() || "/!?".contains(next))
    })
}

/// The targets of the inline links of a page, outside its code blocks and code spans.
fn links(markdown: &str) -> Vec<String> {
    prose(markdown)
        .into_iter()
        .flat_map(|line| {
            let text = outside_code_spans(line);
            link_targets(&text)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The targets of the inline links of `line`: what is between `](` and `)`.
fn link_targets(line: &str) -> Vec<&str> {
    line.split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .collect()
}

#[test]
fn docs_lists_every_page_in_the_order_of_the_summary() {
    let listed = stdout(&["docs"]);

    let entries = summary_entries();
    let lines: Vec<&str> = listed.lines().collect();
    assert_eq!(lines.len(), entries.len(), "{listed}");
    for (line, entry) in lines.iter().zip(&entries) {
        let markdown = read(&page_file(&entry.topic));
        assert_eq!(line.split_whitespace().next(), Some(entry.topic.as_str()));
        assert!(
            line.ends_with(&format!("  {}: {}", entry.title, summary(&markdown))),
            "{line}"
        );
    }
}

#[test]
fn docs_prints_the_markdown_of_a_page_as_it_is() {
    for topic in [
        "concepts",
        "getting-started/installation",
        "agents/json-output",
    ] {
        assert_eq!(stdout(&["docs", topic]), read(&page_file(topic)), "{topic}");
    }
}

#[test]
fn docs_json_lists_the_topic_title_and_summary_of_every_page() {
    let listed = document(&["docs", "--json"]);

    let expected: Vec<Value> = summary_entries()
        .iter()
        .map(|entry| {
            let markdown = read(&page_file(&entry.topic));
            json!({"topic": entry.topic, "title": entry.title, "summary": summary(&markdown)})
        })
        .collect();
    assert_eq!(listed, json!({"schema": 1, "topics": expected}));
}

#[test]
fn docs_json_of_a_page_carries_its_markdown() {
    let page = document(&["docs", "concepts", "--json"]);

    let markdown = read(&page_file("concepts"));
    assert_eq!(
        page,
        json!({
            "schema": 1,
            "topic": "concepts",
            "title": "Concepts",
            "summary": summary(&markdown),
            "markdown": markdown,
        })
    );
}

#[test]
fn an_unknown_topic_fails_and_says_how_to_list_the_topics() {
    octok(&["docs", "nope"])
        .assert()
        .code(3)
        .stdout("")
        .stderr("error: topic `nope` not found: list the topics with `octok docs`\n");

    let out = octok(&["docs", "concepts.md", "--json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let failure: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(failure["schema"], 1);
    assert_eq!(failure["error"]["code"], "topic_not_found");
    assert_eq!(failure["error"]["topic"], "concepts.md");
}

#[test]
fn the_command_reference_in_the_book_is_the_one_octok_renders() {
    let rendered = stdout(&["docs", COMMAND_REFERENCE]);

    assert!(
        rendered == read(&page_file(COMMAND_REFERENCE)),
        "`docs/src/{COMMAND_REFERENCE}.md` is out of date: run `make docs-reference`"
    );
}

#[test]
fn the_command_reference_covers_every_command_argument_and_option() {
    let reference = stdout(&["docs", COMMAND_REFERENCE]);
    let global = documented(reference_section(&reference, "octok"));
    let mut commands: Vec<Vec<String>> = vec![Vec::new()];
    let mut checked = 0;

    while let Some(path) = commands.pop() {
        let mut args: Vec<&str> = path.iter().map(String::as_str).collect();
        args.push("--help");
        let help = stdout(&args);
        let command = format!("octok {}", path.join(" "));
        let command = command.trim_end();
        let documented = documented(reference_section(&reference, command));
        for item in arguments_and_options(&help) {
            assert!(
                documented.contains(&item) || global.contains(&item),
                "`{command}`: `{item}` is not in the command reference"
            );
            checked += 1;
        }
        for name in subcommands(&help) {
            commands.push(path.iter().cloned().chain([name]).collect());
        }
    }

    assert!(
        checked > 40,
        "only {checked} arguments and options were found"
    );
}

#[test]
fn every_command_argument_and_option_of_the_reference_has_help_text() {
    let reference = stdout(&["docs", COMMAND_REFERENCE]);

    let items: Vec<&str> = reference
        .lines()
        .filter(|line| line.starts_with("- `"))
        .collect();
    assert!(!items.is_empty());
    for item in items {
        let (_, help) = item.split_once("`: ").unwrap_or_default();
        assert!(!help.trim().is_empty(), "no help text: {item}");
    }
    for section in reference.split("\n## ").skip(1) {
        let about = section.lines().nth(2).unwrap_or_default();
        assert!(
            !about.is_empty() && !about.starts_with("```"),
            "no description: {}",
            section.lines().next().unwrap_or_default()
        );
    }
}

#[test]
fn every_page_is_in_the_summary_and_every_entry_of_the_summary_has_a_page() {
    let entries = summary_entries();
    let listed: BTreeSet<String> = entries.iter().map(|entry| entry.topic.clone()).collect();

    assert_eq!(listed.len(), entries.len(), "a page is listed twice");
    assert_eq!(listed, page_topics());
}

#[test]
fn every_page_starts_with_its_title_and_a_one_line_summary() {
    for entry in summary_entries() {
        let markdown = read(&page_file(&entry.topic));
        let lines: Vec<&str> = markdown.lines().collect();
        let topic = &entry.topic;

        assert_eq!(
            lines.first().copied(),
            Some(format!("# {}", entry.title).as_str()),
            "{topic}"
        );
        assert_eq!(
            lines.get(1).copied(),
            Some(""),
            "{topic}: line 2 is not empty"
        );
        let summary = lines.get(2).copied().unwrap_or_default();
        assert!(
            !summary.trim().is_empty() && !summary.starts_with(['#', '`', '-', '|', '>', '[']),
            "{topic}: line 3 is not a summary: {summary:?}"
        );
        assert!(
            lines.get(3).is_none_or(|line| line.is_empty()),
            "{topic}: the summary is longer than one line"
        );
    }
}

#[test]
fn no_page_uses_an_mdbook_directive() {
    for entry in summary_entries() {
        let markdown = read(&page_file(&entry.topic));
        assert!(
            !markdown.contains("{{#"),
            "{}: mdBook directive",
            entry.topic
        );
    }
}

#[test]
fn every_link_between_pages_leads_to_a_page() {
    let pages: BTreeSet<PathBuf> = page_topics()
        .iter()
        .map(|topic| page_file(topic).canonicalize().unwrap())
        .collect();
    for entry in summary_entries() {
        let file = page_file(&entry.topic);
        let markdown = read(&file);
        for target in links(&markdown) {
            let path = target.split('#').next().unwrap_or_default();
            if path.is_empty() || target.contains("://") || target.starts_with("mailto:") {
                continue;
            }
            let linked = file.parent().unwrap().join(path).canonicalize();
            assert!(
                linked.is_ok_and(|linked| pages.contains(&linked)),
                "{}: `{target}` is not a page of the book",
                entry.topic
            );
        }
    }
}

#[test]
fn no_page_has_html_or_a_placeholder_outside_code() {
    let mut found = Vec::new();
    for entry in summary_entries() {
        let markdown = read(&page_file(&entry.topic));
        for line in prose(&markdown) {
            if has_html(&outside_code_spans(line)) {
                found.push(format!("{}: {line}", entry.topic));
            }
        }
    }

    assert!(
        found.is_empty(),
        "a page is plain CommonMark: put each `<placeholder>` in a code span, and link with `[text](url)`:\n{}",
        found.join("\n")
    );
}

#[test]
fn html_is_found_outside_code_blocks_and_code_spans_only() {
    let markdown = "Run `octok rm <branch>` or ``a `<b>` c``.\n\
                    ````text\n```console\n$ octok checkout <branch>\n```\n````\n\
                    A branch <name>, then a < b.";

    let found: Vec<&str> = prose(markdown)
        .into_iter()
        .filter(|line| has_html(&outside_code_spans(line)))
        .collect();

    assert_eq!(found, ["A branch <name>, then a < b."]);
    for html in [
        "</b>",
        "<!-- note -->",
        "<https://example.com>",
        "an unclosed `<b>",
    ] {
        assert!(has_html(&outside_code_spans(html)), "{html}");
    }
    for text in ["`<branch>`", "a < b", "\\<branch>"] {
        assert!(!has_html(&outside_code_spans(text)), "{text}");
    }
}

#[test]
fn the_links_of_a_page_are_read_outside_code() {
    let markdown = "See [the project](../commands/project.md#init), not `[text](url)`.\n\
                    ```json\n{\"a\":[1](2)}\n```\n\
                    Then [JSON](json-output.md) and [site](https://example.com).";

    let targets = links(markdown);

    assert_eq!(
        targets,
        [
            "../commands/project.md#init",
            "json-output.md",
            "https://example.com"
        ]
    );
}
