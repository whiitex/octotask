//! Checks the plugin in `plugins/octotask/` against the real `octok` and against what each
//! agent asks of a plugin, so a renamed command or a moved file fails the build instead of
//! leaving a stale instruction for the agents that read it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

/// The name of the plugin, of its one skill and of the catalogs that list it.
const SKILL: &str = "octotask";

/// The catalogs that tell an agent where the plugin is. Claude Code and the `skills`
/// installer read the first one. Codex reads the second one, and no other once it is there.
const MARKETPLACES: [&str; 2] = [
    ".claude-plugin/marketplace.json",
    ".agents/plugins/marketplace.json",
];

/// The schema a `plugin.json` must name for Codex to read it at the root of a plugin.
const PLUGIN_SCHEMA: &str = "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";

/// The fields that schema allows. It refuses any other.
const PLUGIN_FIELDS: [&str; 10] = [
    "$schema",
    "name",
    "version",
    "description",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "extensions",
];

/// What ends a command written inside a sentence, a code span or a shell line.
const COMMAND_END: [char; 7] = ['`', '|', ';', '&', ')', '"', '\n'];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plugin_dir() -> PathBuf {
    repo_root().join("plugins").join(SKILL)
}

/// Claude Code and Codex both look for the skills of a plugin in its `skills` folder.
fn skill_dir() -> PathBuf {
    plugin_dir().join("skills").join(SKILL)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&read(path)).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The pages an agent reads: the skill, and the page of the book it reads with `octok docs`.
fn skill_pages() -> Vec<PathBuf> {
    vec![
        skill_dir().join("SKILL.md"),
        repo_root().join("docs/src/agents/json-output.md"),
    ]
}

/// Every topic of `octok docs <topic>` written in `text`, placeholders aside.
fn docs_topics(text: &str) -> BTreeSet<&str> {
    text.split("octok docs ")
        .skip(1)
        .filter_map(|rest| rest.split(COMMAND_END).next()?.split_whitespace().next())
        .filter(|topic| !topic.starts_with(['<', '[', '-']))
        .collect()
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

    for link in links {
        assert!(
            skill_dir().join(link).is_file(),
            "`{link}` is not in the skill"
        );
    }
}

#[test]
fn every_page_the_skill_sends_to_is_one_octok_docs_prints() {
    let skill = read(&skill_dir().join("SKILL.md"));
    let topics = docs_topics(&skill);

    assert!(
        topics.contains("agents/json-output"),
        "the skill sends to the JSON reference"
    );
    for topic in topics {
        Command::new(assert_cmd::cargo::cargo_bin!("octok"))
            .args(["docs", topic])
            .assert()
            .success();
    }
}

#[test]
fn every_marketplace_points_at_the_plugin() {
    for marketplace in MARKETPLACES {
        let catalog = read_json(&repo_root().join(marketplace));
        assert_eq!(catalog["name"], SKILL, "{marketplace}");
        let plugins = catalog["plugins"].as_array().unwrap();
        assert_eq!(plugins.len(), 1, "{marketplace}");
        assert_eq!(plugins[0]["name"], SKILL, "{marketplace}");

        // The folder is written alone, or as the `path` of a source that says its kind.
        let source = &plugins[0]["source"];
        let path = source.as_str().or_else(|| source["path"].as_str());
        let path = path.unwrap_or_else(|| panic!("{marketplace}: the source names no folder"));
        let folder = path
            .strip_prefix("./")
            .unwrap_or_else(|| panic!("{marketplace}: `{path}` does not start with `./`"));
        assert_eq!(
            repo_root().join(folder).canonicalize().unwrap(),
            plugin_dir().canonicalize().unwrap(),
            "{marketplace}: `{path}` is not the folder of the plugin"
        );
    }
}

#[test]
fn the_codex_marketplace_says_how_the_plugin_is_installed() {
    let catalog = read_json(&repo_root().join(".agents/plugins/marketplace.json"));
    let plugin = &catalog["plugins"][0];

    assert_eq!(plugin["source"]["source"], "local");
    assert_eq!(plugin["policy"]["installation"], "AVAILABLE");
    assert_eq!(plugin["policy"]["authentication"], "ON_INSTALL");
    let category = plugin["category"].as_str().unwrap_or_default();
    assert!(!category.is_empty(), "the plugin has no category");
}

#[test]
fn the_plugin_manifest_follows_the_agent_plugins_format() {
    let manifest = read_json(&plugin_dir().join("plugin.json"));

    assert_eq!(manifest["$schema"], PLUGIN_SCHEMA);
    assert_eq!(manifest["name"], SKILL);
    for field in manifest.as_object().unwrap().keys() {
        assert!(
            PLUGIN_FIELDS.contains(&field.as_str()),
            "`{field}` is not a field of the format"
        );
    }
}

#[test]
fn the_plugin_has_the_version_of_octok() {
    let manifest = read_json(&plugin_dir().join("plugin.json"));

    assert_eq!(
        manifest["version"],
        env!("CARGO_PKG_VERSION"),
        "`plugins/octotask/plugin.json` must carry the version of `Cargo.toml`"
    );
}
