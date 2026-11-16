//! The questions `octok` asks at a terminal for what the flags leave out.
//! An empty answer keeps the default shown between brackets.

use std::io;

use octotask_core::{
    Location, OriginBranches, Project, Result, Ssh, resolve_location, ssh_config_hosts,
};

use crate::utils::ask;

/// Base branch of a new project when nothing else is chosen.
pub(crate) const DEFAULT_BASE: &str = "staging";

/// How many machines are listed before the location is asked.
const LISTED_MACHINES: usize = 15;

/// How many branches are listed after the default one and the staging ones.
const LISTED_OTHER_BRANCHES: usize = 5;

/// Names teams give to the branch that collects the work before a release, the most common first.
const STAGING_NAMES: [&str; 10] = [
    "staging",
    "stage",
    "stg",
    "develop",
    "development",
    "dev",
    "next",
    "integration",
    "preprod",
    "pre-prod",
];

/// What `octok repo add` registers besides the url. `None` is a flag that was not passed.
pub(crate) struct RepoSettings {
    pub(crate) key: Option<String>,
    pub(crate) alias: Option<String>,
    pub(crate) base: Option<String>,
    pub(crate) location: Option<Location>,
}

/// Asks for the alias and the location of a new repo when the flags left them out.
/// The name is never asked: it is the one the url gives, or `--name`, and a name that cannot
/// be used fails before any question. The base branch is asked later, by [`ask_repo_base`].
pub(crate) fn ask_repo_settings(
    project: &Project,
    url: &str,
    settings: &mut RepoSettings,
) -> Result<()> {
    let mut draft = project.clone();
    let key = draft.add_repo(url, settings.key.as_deref(), None)?;
    if settings.alias.is_none() {
        settings.alias = ask_alias(&draft, &key)?;
    }
    if settings.location.is_none() {
        let default = format!("{}, project default", project_location(project));
        settings.location = ask_location(&default)?;
    }
    Ok(())
}

/// Asks for the base branch of a new repo once it is cloned, after listing the branches
/// of its remote. `None` keeps the base of the project.
pub(crate) fn ask_repo_base(project: &Project, branches: &OriginBranches) -> Option<String> {
    print_branches(branches);
    let default = format!("{}, project default", project.base);
    let base = ask_until("Base branch", &default, |answer| {
        Project::new(&project.name, answer).map(|_| answer.to_string())
    });
    base.ok().flatten()
}

/// Asks for the base branch of a new project.
pub(crate) fn ask_project_base(name: &str) -> Result<Option<String>> {
    ask_until("Base branch", DEFAULT_BASE, |answer| {
        Project::new(name, answer).map(|_| answer.to_string())
    })
}

/// Asks where repos live. An answer that names another machine is tried at once,
/// and what ssh uses to reach it is shown.
pub(crate) fn ask_location(default: &str) -> Result<Option<Location>> {
    let machines = ssh_config_hosts();
    let listed = &machines[..machines.len().min(LISTED_MACHINES)];
    print_list(
        "Machines in ~/.ssh/config:",
        listed,
        machines.len() - listed.len(),
    );
    ask_until("Location", default, |answer| {
        let location = resolve_location(answer)?;
        if let Some(host) = location.host() {
            eprintln!("{}", ssh_summary(host)?);
        }
        Ok(location)
    })
}

/// The key is shown as the default alias, and answering it saves no alias:
/// the repo already answers to its key.
fn ask_alias(draft: &Project, key: &str) -> Result<Option<String>> {
    let alias = ask_until("Alias", key, |answer| {
        if answer != key {
            draft.clone().set_alias(key, answer)?;
        }
        Ok(answer.to_string())
    })?;
    Ok(alias.filter(|alias| alias != key))
}

/// Asks until `accept` takes the answer, showing why it refused the others.
/// `None` is an empty answer, or an input that ended: the default.
fn ask_until<T>(
    question: &str,
    default: &str,
    mut accept: impl FnMut(&str) -> Result<T>,
) -> Result<Option<T>> {
    loop {
        let answer = ask(question, default, io::stdin().lock(), io::stderr());
        let Some(answer) = answer.filter(|answer| !answer.is_empty()) else {
            return Ok(None);
        };
        match accept(&answer) {
            Ok(value) => return Ok(Some(value)),
            Err(refused) => eprintln!("{refused}"),
        }
    }
}

/// The default branch of the remote first, then the staging ones, then the few others
/// with the most recent commits.
fn print_branches(branches: &OriginBranches) {
    let default = branches.default.as_deref();
    let rest = || {
        let all = branches.newest_first.iter().map(String::as_str);
        all.filter(move |branch| Some(*branch) != default)
    };
    let is_staging = |branch: &str| {
        STAGING_NAMES
            .iter()
            .any(|name| branch.eq_ignore_ascii_case(name))
    };
    let staging = STAGING_NAMES
        .iter()
        .flat_map(|name| rest().filter(move |branch| branch.eq_ignore_ascii_case(name)));
    let others = rest()
        .filter(|branch| !is_staging(branch))
        .take(LISTED_OTHER_BRANCHES);
    let lines: Vec<String> = default
        .map(|branch| format!("{branch} (default)"))
        .into_iter()
        .chain(staging.chain(others).map(str::to_string))
        .collect();
    let total = rest().count() + usize::from(default.is_some());
    print_list("Branches on the remote:", &lines, total - lines.len());
}

/// Prints nothing for an empty list. `hidden` is how many more items there are.
fn print_list(title: &str, items: &[String], hidden: usize) {
    if items.is_empty() {
        return;
    }
    eprintln!("{title}");
    for item in items {
        eprintln!("  {item}");
    }
    if hidden > 0 {
        eprintln!("  ... and {hidden} more");
    }
}

fn ssh_summary(host: &str) -> Result<String> {
    let settings = Ssh::new(host)?.settings()?;
    let keys: Vec<String> = settings
        .keys
        .iter()
        .map(|key| key.display().to_string())
        .collect();
    let keys = match keys.as_slice() {
        [] => "no key file".to_string(),
        [key] => format!("key {key}"),
        keys => format!("keys {}", keys.join(", ")),
    };
    Ok(format!(
        "ssh reaches `{host}` as {}@{}, port {}, {keys}",
        settings.user, settings.hostname, settings.port
    ))
}

fn project_location(project: &Project) -> &Location {
    static THIS_MACHINE: Location = Location::Local;
    project.location.as_ref().unwrap_or(&THIS_MACHINE)
}
