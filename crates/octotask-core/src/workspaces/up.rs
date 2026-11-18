use std::io::{BufRead, BufReader, PipeReader};
use std::process::ExitStatus;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use super::workspace::{Member, chosen_keys, members_of_existing};
use crate::adapters::{Commands, Lifeline, Supervised, supervise};
use crate::{Error, Home, Project, Result};

/// What the commands of a workspace do while `Running::wait` follows them.
#[derive(Debug, Clone, Copy)]
pub enum UpEvent<'a> {
    /// One line shown for a repo: a command octok runs, after `$ `, or what it wrote to
    /// stdout or stderr.
    Output {
        repo: &'a str,
        line: &'a str,
    },
    Exited {
        repo: &'a str,
        status: ExitStatus,
    },
}

/// The commands of a workspace, started. Dropping it stops them.
pub struct Running {
    services: Vec<Service>,
    messages: Receiver<Message>,
    sender: Sender<Message>,
}

/// Asks the commands of a workspace to stop, from any thread.
pub struct Stopper(Sender<Message>);

struct Service {
    repo: String,
    /// `None` once the commands were asked to stop, or have ended.
    lifeline: Option<Lifeline>,
}

enum Message {
    Output { service: usize, line: String },
    Exited { service: usize, status: ExitStatus },
    Stop,
}

/// Starts the repos of the workspace in their checkouts, each with its `run` command
/// after the setup that is due. `repos` are keys or aliases: none means every repo that
/// has a `run` command. Nothing is started unless every one of them can be.
pub fn start_workspace(
    home: &Home,
    project: &Project,
    workspace: &str,
    repos: &[&str],
) -> Result<Running> {
    let to_run = repos_to_run(home, project, workspace, repos)?;
    let (sender, messages) = mpsc::channel();
    let mut services = Vec::new();
    for (member, commands) in &to_run {
        let supervised = supervise(&member.location, &member.worktree, commands)?;
        let lifeline = follow(services.len(), supervised, sender.clone());
        services.push(Service {
            repo: member.key.to_string(),
            lifeline: Some(lifeline),
        });
    }
    Ok(Running {
        services,
        messages,
        sender,
    })
}

impl Running {
    /// Keys of the repos that were started.
    pub fn started(&self) -> impl Iterator<Item = &str> {
        self.services.iter().map(|service| service.repo.as_str())
    }

    pub fn stopper(&self) -> Stopper {
        Stopper(self.sender.clone())
    }

    /// Follows the commands until all of them have ended, reporting what they print and
    /// how they end. Returns whether every repo ended well.
    pub fn wait(mut self, mut on_event: impl FnMut(UpEvent)) -> bool {
        let mut running = self.services.len();
        let mut all_succeeded = true;
        while running > 0 {
            // Never fails: this `Running` holds a sender.
            let Ok(message) = self.messages.recv() else {
                break;
            };
            match message {
                Message::Output { service, line } => on_event(UpEvent::Output {
                    repo: &self.services[service].repo,
                    line: &line,
                }),
                Message::Exited { service, status } => {
                    running -= 1;
                    all_succeeded &= status.success();
                    self.services[service].lifeline = None;
                    on_event(UpEvent::Exited {
                        repo: &self.services[service].repo,
                        status,
                    });
                }
                Message::Stop => {
                    for service in &mut self.services {
                        service.lifeline = None;
                    }
                }
            }
        }
        all_succeeded
    }
}

impl Stopper {
    pub fn stop(&self) {
        // Nobody listens once the commands have ended.
        let _ = self.0.send(Message::Stop);
    }
}

/// The repos to start with their commands, once it is sure that each one has a checkout.
fn repos_to_run<'a>(
    home: &Home,
    project: &'a Project,
    workspace: &str,
    repos: &[&str],
) -> Result<Vec<(Member<'a>, Commands<'a>)>> {
    let members = members_of_existing(home, project, workspace)?;
    let chosen = chosen_keys(project, repos)?;
    for key in chosen.iter().flatten() {
        if project.effective_run(key, workspace).is_none() {
            return Err(Error::NoRunCommand(key.to_string()));
        }
    }
    let mut to_run = Vec::new();
    for member in members {
        let Some(run) = project.effective_run(member.key, workspace) else {
            continue;
        };
        if chosen
            .as_ref()
            .is_some_and(|keys| !keys.contains(&member.key))
        {
            continue;
        }
        if !member.has_worktree()? {
            return Err(Error::NoWorktree {
                repo: member.key.to_string(),
                workspace: workspace.to_string(),
            });
        }
        let setup = member.repo.setup.as_deref();
        to_run.push((member, Commands { run, setup }));
    }
    if to_run.is_empty() {
        return Err(Error::NothingToRun(workspace.to_string()));
    }
    Ok(to_run)
}

/// Reports the output and the end of the commands of a repo, from a thread of its own.
/// Returns what stops them when dropped.
fn follow(service: usize, supervised: Supervised, sender: Sender<Message>) -> Lifeline {
    let Supervised {
        lifeline,
        output,
        mut process,
    } = supervised;
    thread::spawn(move || {
        // Sending fails once nobody follows the commands: their output is still read to
        // its end, so they never block on it.
        read_lines(output, |line| {
            let _ = sender.send(Message::Output { service, line });
        });
        let status = process.wait().expect("the command is waited for once");
        let _ = sender.send(Message::Exited { service, status });
    });
    lifeline
}

fn read_lines(output: PipeReader, mut on_line: impl FnMut(String)) {
    let mut output = BufReader::new(output);
    let mut bytes = Vec::new();
    while output
        .read_until(b'\n', &mut bytes)
        .is_ok_and(|read| read > 0)
    {
        let line = String::from_utf8_lossy(&bytes);
        on_line(line.trim_end_matches(['\n', '\r']).to_string());
        bytes.clear();
    }
}
