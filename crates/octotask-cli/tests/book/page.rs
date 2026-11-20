//! The `console` examples of a Markdown page, and the page with their output rewritten.

use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Starts a line that `[..]` stands in for, in part or in whole.
const WILDCARD: &str = "[..]";

pub struct Page {
    pub path: PathBuf,
    lines: Vec<String>,
    pub examples: Vec<Example>,
}

/// One `$ ` line of a `console` block and the output written under it.
pub struct Example {
    pub command: String,
    /// 1-based, as an editor shows it.
    pub line_number: usize,
    pub expected: Vec<String>,
    /// The lines of the page that hold `expected`: blank lines after it are not part of it.
    output_lines: Range<usize>,
    indent: String,
}

impl Page {
    pub fn read(path: &Path) -> Self {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let examples = console_blocks(&lines)
            .into_iter()
            .flat_map(|block| examples_of(&lines, block))
            .collect();
        Self {
            path: path.to_path_buf(),
            lines,
            examples,
        }
    }

    /// The text of the page with `outputs[i]` as the output of `examples[i]`.
    pub fn rewritten(&self, outputs: &[Vec<String>]) -> String {
        let mut lines = self.lines.clone();
        for (example, output) in self.examples.iter().zip(outputs).rev() {
            let indented = output.iter().map(|line| indent(&example.indent, line));
            lines.splice(example.output_lines.clone(), indented);
        }
        lines.join("\n") + "\n"
    }
}

impl Example {
    pub fn matches(&self, actual: &[String]) -> bool {
        self.expected.len() == actual.len()
            && self
                .expected
                .iter()
                .zip(actual)
                .all(|(expected, actual)| line_matches(expected, actual))
    }

    /// `actual`, keeping each written line that still matches, so a `[..]` stays.
    pub fn updated(&self, actual: Vec<String>) -> Vec<String> {
        if self.expected.len() != actual.len() {
            return actual;
        }
        self.expected
            .iter()
            .zip(actual)
            .map(|(expected, actual)| {
                if line_matches(expected, &actual) {
                    expected.clone()
                } else {
                    actual
                }
            })
            .collect()
    }
}

/// `actual` matches when it reads as `expected`, with each `[..]` standing for any text.
fn line_matches(expected: &str, actual: &str) -> bool {
    let mut parts = expected.split(WILDCARD);
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = actual.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        match rest.find(part) {
            Some(start) => rest = &rest[start + part.len()..],
            None => return false,
        }
    }
    rest.ends_with(last)
}

/// A fenced block of the page whose info string is `console`.
struct Block {
    /// The lines between the fences.
    body: Range<usize>,
    indent: String,
}

/// The `console` blocks, skipping any `console` fence written inside another block.
fn console_blocks(lines: &[String]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut open: Option<(Fence, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        match &open {
            None => open = Fence::opening(line).map(|fence| (fence, index)),
            Some((fence, start)) if fence.is_closed_by(line) => {
                if fence.info == "console" {
                    blocks.push(Block {
                        body: start + 1..index,
                        indent: fence.indent.clone(),
                    });
                }
                open = None;
            }
            Some(_) => {}
        }
    }
    blocks
}

fn examples_of(lines: &[String], block: Block) -> Vec<Example> {
    let mut examples: Vec<Example> = Vec::new();
    for index in block.body {
        let line = lines[index]
            .strip_prefix(&block.indent)
            .unwrap_or(lines[index].trim_start());
        if let Some(command) = line.strip_prefix("$ ") {
            examples.push(Example {
                command: command.to_string(),
                line_number: index + 1,
                expected: Vec::new(),
                output_lines: index + 1..index + 1,
                indent: block.indent.clone(),
            });
        } else if let Some(example) = examples.last_mut() {
            example.expected.push(line.trim_end().to_string());
        }
    }
    for example in &mut examples {
        while example.expected.last().is_some_and(String::is_empty) {
            example.expected.pop();
        }
        let start = example.output_lines.start;
        example.output_lines = start..start + example.expected.len();
    }
    examples
}

fn indent(indent: &str, line: &str) -> String {
    if line.is_empty() {
        String::new()
    } else {
        format!("{indent}{line}")
    }
}

struct Fence {
    marker: char,
    length: usize,
    info: String,
    indent: String,
}

impl Fence {
    fn opening(line: &str) -> Option<Self> {
        let text = line.trim_start();
        let marker = text.chars().next().filter(|c| *c == '`' || *c == '~')?;
        let length = text.chars().take_while(|c| *c == marker).count();
        (length >= 3).then(|| Self {
            marker,
            length,
            info: text[length..].trim().to_string(),
            indent: line[..line.len() - text.len()].to_string(),
        })
    }

    fn is_closed_by(&self, line: &str) -> bool {
        let text = line.trim();
        text.len() >= self.length && text.chars().all(|c| c == self.marker)
    }
}
