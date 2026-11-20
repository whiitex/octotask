use std::fs;
use std::path::{Path, PathBuf};

use crate::config::validate_host;

/// How many `Include` levels are followed, so two files that include each other end.
const INCLUDE_DEPTH: usize = 8;

/// The hosts `~/.ssh/config` names one by one, in its order: the machines a location can be
/// written with. Patterns such as `*.example.com` are left out, and so is what `Match` selects.
///
/// It only suggests names, so a file that cannot be read gives no host instead of an error.
pub fn ssh_config_hosts() -> Vec<String> {
    let mut hosts = Vec::new();
    if let Some(home) = std::env::home_dir() {
        let ssh_dir = home.join(".ssh");
        let dirs = Dirs {
            home: &home,
            ssh: &ssh_dir,
        };
        collect_hosts(&ssh_dir.join("config"), dirs, INCLUDE_DEPTH, &mut hosts);
    }
    hosts
}

#[derive(Clone, Copy)]
struct Dirs<'a> {
    home: &'a Path,
    ssh: &'a Path,
}

fn collect_hosts(file: &Path, dirs: Dirs, depth: usize, hosts: &mut Vec<String>) {
    let Ok(text) = fs::read_to_string(file) else {
        return;
    };
    for line in text.lines() {
        let mut words = words(line).into_iter();
        let Some(keyword) = words.next() else {
            continue;
        };
        match keyword.to_ascii_lowercase().as_str() {
            "host" => {
                for host in words.filter(|host| names_one_host(host)) {
                    if !hosts.contains(&host) {
                        hosts.push(host);
                    }
                }
            }
            "include" if depth > 0 => {
                for included in words.flat_map(|pattern| included_files(&pattern, dirs)) {
                    collect_hosts(&included, dirs, depth - 1, hosts);
                }
            }
            _ => {}
        }
    }
}

/// The words of one line: separated by blanks, or by the `=` after the keyword, with
/// double quotes keeping a blank inside a word. A comment line has none.
fn words(line: &str) -> Vec<String> {
    let line = line.trim();
    if line.starts_with('#') {
        return Vec::new();
    }
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for c in line.chars() {
        let ends_keyword = c == '=' && words.is_empty();
        if c == '"' {
            quoted = !quoted;
        } else if !quoted && (c.is_whitespace() || ends_keyword) {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn names_one_host(pattern: &str) -> bool {
    !pattern.contains(['*', '?', '!']) && validate_host(pattern).is_ok()
}

/// The files an `Include` names: relative to `~/.ssh`, with `~` for the home, and with
/// `*` and `?` in the file name only.
fn included_files(pattern: &str, dirs: Dirs) -> Vec<PathBuf> {
    let path = match pattern.strip_prefix("~/") {
        Some(under_home) => dirs.home.join(under_home),
        None => dirs.ssh.join(pattern),
    };
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let (Some(parent), Some(name)) = (path.parent(), name) else {
        return Vec::new();
    };
    if !name.contains(['*', '?']) {
        return vec![path];
    }
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| wildcard_matches(&name, &entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

fn wildcard_matches(pattern: &str, name: &str) -> bool {
    let mut pattern_chars = pattern.chars();
    let Some(first) = pattern_chars.next() else {
        return name.is_empty();
    };
    let rest = pattern_chars.as_str();
    if first == '*' {
        return name
            .char_indices()
            .map(|(start, _)| start)
            .chain([name.len()])
            .any(|start| wildcard_matches(rest, &name[start..]));
    }
    let mut name_chars = name.chars();
    match name_chars.next() {
        Some(c) if first == '?' || first == c => wildcard_matches(rest, name_chars.as_str()),
        _ => false,
    }
}
