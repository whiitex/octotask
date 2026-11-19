//! Checks `skills/octotask/` against the real `octok`, so a renamed command or option
//! fails the build instead of leaving a stale instruction for the agents that read it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

const SKILL: &str = "octotask";

/// What ends a command written inside a sentence, a code span or a shell line.
const COMMAND_END: [char; 7] = ['`', '|', ';', '&', ')', '"', '\n'];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skill_dir() -> PathBuf {
    repo_root().join("skills").join(SKILL)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The pages an agent reads: the skill and its references.
fn skill_pages() -> Vec<PathBuf> {
    let mut pages = vec![skill_dir().join("SKILL.md")];
    let references = fs::read_dir(skill_dir().join("references")).unwrap();
    pages.extend(references.map(|entry| entry.unwrap().path()));
    pages.sort();
    pages
}

/// Every `octok ...` written in `text`, as the arguments a shell would pass. A
/// placeholder such as `<branch>` becomes a value that `octok` can parse.
fn octok_commands(text: &str) -> BTreeSet<Vec<String>> {
    text.match_indices("octok ")
        .filter(|(start, _)| !is_inside_a_path_or_a_word(&text[..*start]))
        .map(|(start, name)| {
            let rest = &text[start + name.len()..];
            let command = rest.split(COMMAND_END).next().unwrap_or_default();
            let command = command
                .replace("<number>", "12")
                .replace(['<', '>', '[', ']'], "");
            shell_words(&command)
        })
        .filter(|arguments| !arguments.is_empty())
        .collect()
}

/// `before` ends in the middle of a path or of a name, as in `bin/octok` and `my-octok`.
fn is_inside_a_path_or_a_word(before: &str) -> bool {
    before
        .chars()
        .next_back()
        .is_some_and(|last| last.is_alphanumeric() || "/-_.".contains(last))
}

/// Splits at the spaces that are not between single quotes, as a shell does.
fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for character in command.chars() {
        match character {
            '\'' => quoted = !quoted,
            space if space.is_whitespace() && !quoted => {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            }
            other => word.push(other),
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

/// The front matter of a skill: the lines between the first two `---`.
fn front_matter(skill: &str) -> Vec<(&str, &str)> {
    let mut lines = skill.lines();
    assert_eq!(
        lines.next(),
        Some("---"),
        "a skill starts with front matter"
    );
    lines
        .take_while(|line| *line != "---")
        .filter_map(|line| line.split_once(": "))
        .collect()
}

#[test]
fn every_command_in_the_skill_is_one_octok_accepts() {
    let home = TempDir::new().unwrap();
    let mut checked = 0;

    for page in skill_pages() {
        for arguments in octok_commands(&read(&page)) {
            // `--help` comes last: clap refuses an unknown command or option before it.
            // `octok help` is the one command that only prints, and takes no `--help`.
            let only_prints = arguments.first().is_some_and(|command| command == "help");
            let output = Command::new(assert_cmd::cargo::cargo_bin!("octok"))
                .env("OCTOTASK_HOME", home.path())
                .args(&arguments)
                .args((!only_prints).then_some("--help"))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}: `octok {}` is not a command octok accepts:\n{}",
                page.display(),
                arguments.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
            checked += 1;
        }
    }

    assert!(checked > 20, "only {checked} commands were found");
    assert!(
        fs::read_dir(home.path()).unwrap().next().is_none(),
        "checking a command must not run it"
    );
}

#[test]
fn the_commands_of_a_page_are_read_as_a_shell_would() {
    let page = "Run `octok status --json`, then `cd \"$(octok checkout <branch>)\"`.\n\
                ```sh\noctok pr review --pr <repo>=<number> | jq .path\n\
                octok repo set web --run 'npm run dev'\n\
                mv octok-linux/octok ~/.local/bin/\n```";

    let commands: Vec<String> = octok_commands(page)
        .into_iter()
        .map(|arguments| arguments.join(" "))
        .collect();

    assert_eq!(
        commands,
        [
            "checkout branch",
            "pr review --pr repo=12",
            "repo set web --run npm run dev",
            "status --json"
        ]
    );
}

#[test]
fn the_skill_follows_the_agent_skills_format() {
    let skill = read(&skill_dir().join("SKILL.md"));
    let fields = front_matter(&skill);
    let field = |name: &str| {
        let found = fields.iter().find(|(field, _)| *field == name);
        found
            .unwrap_or_else(|| panic!("no `{name}` in the front matter"))
            .1
    };

    assert_eq!(field("name"), SKILL, "the name is the one of the folder");
    let description = field("description");
    assert!(
        (1..=1024).contains(&description.chars().count()),
        "the description has {} characters",
        description.chars().count()
    );
    assert!(skill.lines().count() < 500, "the skill is too long");
}

#[test]
fn the_skill_links_only_to_pages_it_ships() {
    let skill = read(&skill_dir().join("SKILL.md"));

    let links: Vec<&str> = skill
        .split("](")
        .skip(1)
        .filter_map(|rest| rest.split(')').next())
        .filter(|link| !link.starts_with("http") && !link.starts_with('#'))
        .map(|link| link.split('#').next().unwrap_or(link))
        .collect();

    assert!(!links.is_empty(), "the skill links to its reference");
    for link in links {
        assert!(
            skill_dir().join(link).is_file(),
            "`{link}` is not in the skill"
        );
    }
}

#[test]
fn the_plugin_marketplace_points_at_the_skill() {
    let marketplace = read(&repo_root().join(".claude-plugin/marketplace.json"));
    let marketplace: serde_json::Value = serde_json::from_str(&marketplace).unwrap();

    let plugins = marketplace["plugins"].as_array().unwrap();
    assert_eq!(plugins.len(), 1);
    assert_eq!(plugins[0]["name"], SKILL);
    let source = plugins[0]["source"].as_str().unwrap();
    assert!(
        repo_root().join(source).join("SKILL.md").is_file(),
        "`{source}` has no SKILL.md"
    );
}
