//! `octok docs`: the pages of the book, built into the binary so that they describe the
//! version that prints them.

use std::borrow::Cow;

use clap::{Arg, Command};
use octotask_core::{Error, Result};
use serde::Serialize;
use serde_json::json;

use crate::utils::{Output, max_width};

/// The page that is rendered from the command line of `octok` instead of read from its
/// file: `docs/src/reference/commands.md` is a copy of it, for the site.
const COMMAND_REFERENCE: &str = "reference/commands";

/// A page of `docs/src`, as `SUMMARY.md` lists it.
struct Page {
    /// Its path under `docs/src`, without `.md`.
    topic: &'static str,
    title: &'static str,
    markdown: &'static str,
}

/// Every page, in the order of `SUMMARY.md`.
static PAGES: &[Page] = include!(concat!(env!("OUT_DIR"), "/pages.rs"));

/// Lists the pages, or prints the one of `topic`. `cli` is the command line of `octok`.
pub(crate) fn show(topic: Option<&str>, cli: &Command, out: Output) -> Result<()> {
    match topic {
        None => list_pages(cli, out),
        Some(topic) => {
            let page = PAGES
                .iter()
                .find(|page| page.topic == topic)
                .ok_or_else(|| Error::TopicNotFound(topic.to_string()))?;
            print_page(page, cli, out);
        }
    }
    Ok(())
}

/// What `octok docs` says about a page.
#[derive(Serialize)]
struct Topic<'a> {
    topic: &'a str,
    title: &'a str,
    summary: &'a str,
}

fn list_pages(cli: &Command, out: Output) {
    let markdowns: Vec<Cow<str>> = PAGES.iter().map(|page| markdown(page, cli)).collect();
    let topics: Vec<Topic> = PAGES
        .iter()
        .zip(&markdowns)
        .map(|(page, markdown)| topic(page, markdown))
        .collect();
    out.result(&json!({"topics": topics}), |_| {
        let width = max_width(topics.iter().map(|topic| topic.topic));
        for topic in &topics {
            println!("{:width$}  {}: {}", topic.topic, topic.title, topic.summary);
        }
    });
}

fn print_page(page: &Page, cli: &Command, out: Output) {
    #[derive(Serialize)]
    struct Document<'a> {
        #[serde(flatten)]
        topic: Topic<'a>,
        markdown: &'a str,
    }
    let markdown = markdown(page, cli);
    let document = Document {
        topic: topic(page, &markdown),
        markdown: &markdown,
    };
    out.result(&document, |document| print!("{}", document.markdown));
}

fn topic<'a>(page: &'a Page, markdown: &'a str) -> Topic<'a> {
    Topic {
        topic: page.topic,
        title: page.title,
        // The line under the title, by the conventions of the book.
        summary: markdown.lines().nth(2).unwrap_or_default(),
    }
}

fn markdown(page: &Page, cli: &Command) -> Cow<'static, str> {
    if page.topic == COMMAND_REFERENCE {
        Cow::Owned(command_reference(cli))
    } else {
        Cow::Borrowed(page.markdown)
    }
}

/// Every command of `cli` with its description, its usage, its arguments and its options,
/// as their `--help` gives them.
fn command_reference(cli: &Command) -> String {
    let mut cli = cli.clone();
    // Gives each command its full name in its usage, and the options it inherits.
    cli.build();
    let mut page = String::from(
        "# Command reference\n\n\
         Every command of `octok` with its arguments and options, as its `--help` describes them.\n\n\
         The options of every command are listed once, under [`octok`](#octok).\n",
    );
    write_command(&mut page, &cli, None);
    page
}

/// Writes the section of `command`, then the ones of its subcommands. `parent` is the
/// full name of the command above it.
fn write_command(page: &mut String, command: &Command, parent: Option<&str>) {
    let name = match parent {
        Some(parent) => format!("{parent} {}", command.get_name()),
        None => command.get_name().to_string(),
    };
    write_description(page, &name, command);
    write_arguments(page, command, parent.is_none());
    let subcommands: Vec<&Command> = command
        .get_subcommands()
        .filter(|subcommand| !is_help(subcommand))
        .collect();
    let items = subcommands
        .iter()
        .map(|subcommand| subcommand_item(&name, subcommand));
    write_list(page, "Commands", items);
    for subcommand in subcommands {
        write_command(page, subcommand, Some(&name));
    }
}

/// The heading, what the command does and its usage.
fn write_description(page: &mut String, name: &str, command: &Command) {
    let about = command
        .get_long_about()
        .or(command.get_about())
        .map(ToString::to_string)
        .unwrap_or_default();
    let usage = command.clone().render_usage().to_string();
    page.push_str(&format!(
        "\n## `{name}`\n\n{}\n\n```text\n{}\n```\n",
        one_sentence_per_line(about.trim_end()),
        usage.trim_end()
    ));
}

/// The options that every command inherits, and `--help` that every command has, are
/// written once, under the `root` command.
fn write_arguments(page: &mut String, command: &Command, root: bool) {
    let of_every_command = |arg: &&Arg| arg.is_global_set() || arg.get_id() == "help";
    let (positionals, options): (Vec<&Arg>, Vec<&Arg>) = command
        .get_arguments()
        .filter(|arg| !arg.is_hide_set())
        .partition(|arg| arg.is_positional());
    let (shared, own): (Vec<&Arg>, Vec<&Arg>) = options.into_iter().partition(of_every_command);
    write_list(page, "Arguments", positionals.into_iter().map(arg_item));
    if root {
        write_list(
            page,
            "Options of every command",
            shared.into_iter().map(arg_item),
        );
    }
    write_list(page, "Options", own.into_iter().map(arg_item));
}

/// A link to the section of a subcommand of `parent`, with what it does.
fn subcommand_item(parent: &str, subcommand: &Command) -> String {
    let name = format!("{parent} {}", subcommand.get_name());
    let about = subcommand
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    // The anchor that mdBook and GitHub give the heading.
    format!("[`{name}`](#{}): {about}", name.replace(' ', "-"))
}

/// Breaks the line after each sentence, as in every page of the book. A sentence ends
/// with a period before a capital letter, so the one of `e.g.` before a code span does not.
fn one_sentence_per_line(text: &str) -> String {
    let mut lines = String::new();
    let mut rest = text;
    while let Some(end) = rest.find(". ") {
        let next = &rest[end + 2..];
        lines.push_str(&rest[..=end]);
        lines.push(if next.starts_with(char::is_uppercase) {
            '\n'
        } else {
            ' '
        });
        rest = next;
    }
    lines.push_str(rest);
    lines
}

/// The `help` command that clap adds.
fn is_help(command: &Command) -> bool {
    command.get_name() == "help"
}

/// A list under `title`, if it has items.
fn write_list(page: &mut String, title: &str, items: impl Iterator<Item = String>) {
    let items: Vec<String> = items.map(|item| format!("- {item}\n")).collect();
    if !items.is_empty() {
        page.push_str(&format!("\n{title}:\n\n{}", items.concat()));
    }
}

/// `` `-p, --project <PROJECT>`: help `` on one line.
fn arg_item(arg: &Arg) -> String {
    let label = match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(_)) => format!("-{short}, {arg}"),
        _ => arg.to_string(),
    };
    let help = arg
        .get_long_help()
        .or(arg.get_help())
        .map(ToString::to_string)
        .unwrap_or_default();
    let help: Vec<&str> = help.split_whitespace().collect();
    format!("`{label}`: {}", help.join(" "))
}
