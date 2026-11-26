//! The values a repo declares for its commands, and the templates that read them.

use std::collections::HashMap;

use super::project::Project;
use crate::{Error, Result};

impl Project {
    /// Env values of a repo in a workspace, templates replaced: undefined or looping ones fail.
    pub fn resolved_env(&self, key: &str, workspace: &str) -> Result<Vec<(String, String)>> {
        let mut resolver = Resolver {
            project: self,
            workspace,
            resolved: HashMap::new(),
            reading: Vec::new(),
        };
        self.effective_env(key, workspace)
            .into_keys()
            .map(|name| Ok((name.to_string(), resolver.value(key, name)?)))
            .collect()
    }
}

/// A piece of a template: text kept as it is, or a variable to replace.
pub(crate) enum Piece<'a> {
    Text(&'a str),
    Variable {
        /// What is written between the braces, without the spaces around it.
        text: &'a str,
        variable: Variable<'a>,
    },
}

pub(crate) enum Variable<'a> {
    ProjectName,
    WorkspaceName,
    WorkspaceSlug,
    /// `repo` is a key or an alias.
    RepoEnv {
        repo: &'a str,
        name: &'a str,
    },
}

/// Resolves the values of one workspace, each once.
struct Resolver<'a> {
    project: &'a Project,
    workspace: &'a str,
    /// By repo key and name.
    resolved: HashMap<(&'a str, &'a str), String>,
    /// The values being resolved, outermost first: one read again is a loop.
    reading: Vec<(&'a str, &'a str)>,
}

impl<'a> Resolver<'a> {
    /// `name` is defined for the repo `key`.
    fn value(&mut self, key: &'a str, name: &'a str) -> Result<String> {
        if let Some(value) = self.resolved.get(&(key, name)) {
            return Ok(value.clone());
        }
        if let Some(start) = self.reading.iter().position(|read| *read == (key, name)) {
            let chain = self.reading[start..].iter().copied().chain([(key, name)]);
            let chain = chain.map(|(key, name)| format!("{key}.{name}")).collect();
            return Err(Error::EnvCycle(chain));
        }
        let template = self.project.effective_env(key, self.workspace)[name];
        let pieces = parse_template(template).map_err(|reason| Error::InvalidEnv {
            repo: key.to_string(),
            name: name.to_string(),
            reason,
        })?;
        self.reading.push((key, name));
        let mut value = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(text) => value.push_str(text),
                Piece::Variable { text, variable } => {
                    let undefined = || Error::UndefinedVariable {
                        repo: key.to_string(),
                        name: name.to_string(),
                        variable: text.to_string(),
                    };
                    value.push_str(&self.variable(variable).ok_or_else(undefined)??);
                }
            }
        }
        self.reading.pop();
        self.resolved.insert((key, name), value.clone());
        Ok(value)
    }

    /// `None` when the variable is not defined.
    fn variable(&mut self, variable: Variable<'a>) -> Option<Result<String>> {
        match variable {
            Variable::ProjectName => Some(Ok(self.project.name.clone())),
            Variable::WorkspaceName => Some(Ok(self.workspace.to_string())),
            Variable::WorkspaceSlug => Some(Ok(slug(self.workspace))),
            Variable::RepoEnv { repo, name } => {
                let key = self.project.resolve_repo(repo).ok()?;
                let env = self.project.effective_env(key, self.workspace);
                let (name, _) = env.get_key_value(name)?;
                Some(self.value(key, name))
            }
        }
    }
}

/// Lowercase, with every character but `a-z`, `0-9`, `_` and `-` replaced by `-`.
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '_' | '-' => c,
            _ => '-',
        })
        .collect()
}

/// Why an env name or value cannot be used, if it cannot.
pub(crate) fn check_env(name: &str, value: &str) -> std::result::Result<(), &'static str> {
    if !is_env_name(name) {
        return Err("must start with a letter or `_` and contain only letters, digits and `_`");
    }
    if value.contains(['\n', '\0']) {
        return Err("must not contain a newline or a NUL character");
    }
    parse_template(value).map(drop)
}

/// Splits a value into text and `{{ variable }}` pieces.
pub(crate) fn parse_template(value: &str) -> std::result::Result<Vec<Piece<'_>>, &'static str> {
    let mut pieces = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            pieces.push(Piece::Text(&rest[..start]));
        }
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or("has `{{` without `}}`")?;
        let text = after[..end].trim();
        let variable = parse_variable(text).ok_or("uses an unknown variable")?;
        pieces.push(Piece::Variable { text, variable });
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest));
    }
    Ok(pieces)
}

fn parse_variable(text: &str) -> Option<Variable<'_>> {
    match text {
        "project.name" => Some(Variable::ProjectName),
        "workspace.name" => Some(Variable::WorkspaceName),
        "workspace.slug" => Some(Variable::WorkspaceSlug),
        // A repo key or alias may hold `.`, an env name never does.
        _ => {
            let (repo, name) = text.strip_prefix("repos.")?.rsplit_once(".env.")?;
            (!repo.is_empty() && is_env_name(name)).then_some(Variable::RepoEnv { repo, name })
        }
    }
}

fn is_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
