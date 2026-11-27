use serde::Serialize;

use super::repos::RepoResult;
use super::workspace::{Member, chosen_keys, members_of_existing, only_chosen};
use crate::{Editor, Error, Home, Project, Result};

/// A repo opened in an editor.
#[derive(Debug, Serialize)]
pub struct Opened {
    /// The worktree, as `host:/path` on another machine.
    pub path: String,
}

/// Opens each chosen repo (keys or aliases, none for all) in its own window of `editor`.
pub fn open_workspace(
    home: &Home,
    project: &Project,
    workspace: &str,
    editor: Editor,
    repos: &[&str],
) -> Result<Vec<RepoResult<Opened>>> {
    let to_open = repos_to_open(home, project, workspace, repos)?;
    let mut report = Vec::new();
    for member in to_open {
        let opened = editor.open(&member.location, &member.worktree);
        if let Err(error @ Error::ToolNotFound(_)) = opened {
            return Err(error);
        }
        report.push(RepoResult {
            key: member.key.to_string(),
            result: opened.map(|()| Opened {
                path: member.shown_worktree().display().to_string(),
            }),
        });
    }
    Ok(report)
}

/// The chosen repos in project order, once it is sure that each one has a worktree.
fn repos_to_open<'a>(
    home: &Home,
    project: &'a Project,
    workspace: &str,
    repos: &[&str],
) -> Result<Vec<Member<'a>>> {
    let members = members_of_existing(home, project, workspace)?;
    let chosen = chosen_keys(project, repos)?;
    let to_open = only_chosen(members, chosen.as_deref());
    for member in &to_open {
        member.require_worktree(workspace)?;
    }
    Ok(to_open)
}
