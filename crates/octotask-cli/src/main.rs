mod prompts;
mod utils;

use std::collections::HashMap;
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use clap::{Args, Parser, Subcommand};
use octotask_core::{
    Error, Gh, Home, Location, NewRepo, NewWorkspace, OriginBranches, Project, Registry,
    RepoChanges, Result, Ssh, Stopper, check_remote_branch, checkout_workspace, create_workspace,
    dev_changes, fetch_repos, locate_workspace, pull_workspace, register_repo, remove_project,
    remove_workspace, resolve_location, review_workspace, save_dev_changes, set_project_location,
    set_workspace_run, start_workspace, status, unregister_repo, update_repo, workspace_name,
};
use serde_json::json;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use prompts::{
    DEFAULT_BASE, RepoSettings, ask_location, ask_project_base, ask_repo_base, ask_repo_settings,
    ask_what_to_review,
};
use utils::{
    LOCAL, Output, ProjectDocument, RepoDocument, Report, confirm, print_dev_changes,
    print_fetch_report, print_projects, print_pull_report, print_repos, print_status,
    print_up_event, print_usage_failure, print_workspace, repo_label, report_exit_code, up_labels,
};

/// Exit code of a command ended by Ctrl-C, as shells report it.
const INTERRUPTED: u8 = 130;

/// octotask: multi-repo workspaces: one command for branches, PRs and services across repos.
#[derive(Parser)]
#[command(name = "octok", version)]
struct Cli {
    /// Project to act on (defaults to the one set with `octok project use`)
    #[arg(short, long, global = true)]
    project: Option<String>,

    /// Print one JSON document on stdout instead of text, and never ask a question
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage projects (groups of repos)
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Manage the repos registered in a project
    #[command(subcommand)]
    Repo(RepoCommand),
    /// Fetch every repo of the project, cloning the ones not cached yet
    Fetch,
    /// Create a workspace: one worktree per repo, on a new branch from the up-to-date base
    ///
    /// On a workspace that already exists, adds the worktrees it lacks: this completes an
    /// interrupted creation and brings in the repos registered later. Refuses a branch
    /// that is already on a remote: `octok checkout <branch>` is for that.
    Branch {
        /// Name of the new branch
        name: String,
        /// Machine where the whole workspace lives, whatever the repos and the project say:
        /// `local`, `<host>` or `<host>:<path>`
        #[arg(long)]
        location: Option<String>,
        /// Create the workspace even if some dev changes no longer apply: the files they
        /// are for are left as they are in git
        #[arg(long)]
        allow_stale_dev: bool,
    },
    /// Delete a workspace: its worktrees, its folder and the local branches
    Rm {
        /// Also discard uncommitted changes and unpushed commits
        #[arg(long)]
        force: bool,
        /// Branch of the workspace
        branch: String,
    },
    /// Print the path of the workspace of a branch, creating it when the branch is on a remote
    ///
    /// Use it as `cd "$(octok checkout feat/login)"`. When the workspace is not there
    /// yet, a repo that has the branch on its remote gets it, and any other repo gets a
    /// new branch from its base. On a workspace that lacks some worktrees, adds them.
    Checkout {
        /// Create a new branch, like `octok branch`
        #[arg(short = 'b')]
        create: bool,
        /// Branch of the workspace
        branch: String,
        /// Machine where the whole workspace lives, whatever the repos and the project say:
        /// `local`, `<host>` or `<host>:<path>`
        #[arg(long)]
        location: Option<String>,
        /// Create the workspace even if some dev changes no longer apply: the files they
        /// are for are left as they are in git
        #[arg(long)]
        allow_stale_dev: bool,
    },
    /// Fetch, then fast-forward each worktree of a workspace to its remote branch
    Pull {
        /// Branch of the workspace (defaults to the workspace of the current directory)
        branch: Option<String>,
    },
    /// Start the repos of a workspace, each with its `run` command, until Ctrl-C
    ///
    /// Every line a repo prints is shown with the name of the repo in front. A repo
    /// without a `run` command is not started.
    Up {
        /// Keys or aliases of the repos to start (defaults to every repo with a `run` command)
        repos: Vec<String>,
        /// Branch of the workspace (defaults to the workspace of the current directory)
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// Manage the dev changes: what every new workspace gets and no commit contains
    #[command(subcommand)]
    Dev(DevCommand),
    /// Work with pull requests
    #[command(subcommand)]
    Pr(PrCommand),
    /// Show the project, its repos and the checkouts of every workspace
    Status,
    /// Type the passphrase of an ssh key once: `ssh-add` asks for it and keeps it
    SetSshPassphrase {
        /// Machine whose key to unlock (defaults to every other machine of the project)
        host: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Create a new project and make it the default
    Init {
        name: String,
        /// Default base branch for the project's repos [default: staging]
        #[arg(long)]
        base: Option<String>,
        /// Machine where the project's repos live: `local` (the default), `<host>` or `<host>:<path>`
        #[arg(long)]
        location: Option<String>,
    },
    /// List projects
    Ls,
    /// Set the default project
    Use { name: String },
    /// Change the settings of the project
    Set {
        #[command(flatten)]
        changes: ProjectSetArgs,
    },
    /// Delete a project, its list of repos and their clones
    Rm {
        /// Also delete its workspaces, with their uncommitted changes and unpushed commits
        #[arg(long)]
        force: bool,
        name: String,
    },
}

#[derive(Subcommand)]
enum DevCommand {
    /// Save the dev changes of a workspace: what its checkouts have and git does not
    ///
    /// For each repo: every change that is not committed, and every file git does not
    /// track. What was saved before for the repo is replaced.
    Save {
        /// Keys or aliases of the repos (defaults to every repo of the workspace)
        repos: Vec<String>,
        /// A file that git ignores, to save all the same, e.g. `.env`: can be repeated
        #[arg(long = "file", value_name = "PATH")]
        files: Vec<String>,
        /// Branch of the workspace (defaults to the workspace of the current directory)
        #[arg(short, long)]
        branch: Option<String>,
    },
    /// Show the dev changes saved for the repos
    Show {
        /// Keys or aliases of the repos (defaults to every repo of the project)
        repos: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PrCommand {
    /// Create a workspace to try pull requests, with each repo on the branch you choose
    ///
    /// A repo given with `--pr` gets the branch of that pull request, one given with
    /// `--branch` gets that branch, and any other repo gets a new branch from its base.
    /// Without `--pr` and `--branch`, asks at a terminal what to take for each repo.
    /// Prints the path of the workspace.
    ///
    /// Pull requests are read with `gh`, which must be signed in. One that comes from a
    /// fork is refused. What the workspace reviews is remembered until `octok rm`.
    Review(ReviewArgs),
}

#[derive(Args)]
struct ReviewArgs {
    /// A pull request to review, as the key or alias of its repo and its number, e.g.
    /// `backend=12`: can be repeated
    #[arg(long = "pr", value_name = "REPO=NUMBER", value_parser = repo_and_number)]
    pull_requests: Vec<(String, u64)>,
    /// A branch to review, as the key or alias of its repo and its name, e.g.
    /// `agent=fix/prompt`: can be repeated
    #[arg(long = "branch", value_name = "REPO=BRANCH", value_parser = repo_and_branch)]
    branches: Vec<(String, String)>,
    /// Name of the workspace, and of the branch of the repos left on their base (defaults
    /// to `pr-<repo>-<number>` of the first pull request, else to the first branch, in
    /// the order of the repos of the project)
    #[arg(long)]
    name: Option<String>,
    /// Machine where the whole workspace lives, whatever the repos and the project say:
    /// `local`, `<host>` or `<host>:<path>`
    #[arg(long)]
    location: Option<String>,
    /// Create the workspace even if some dev changes no longer apply: the files they
    /// are for are left as they are in git
    #[arg(long)]
    allow_stale_dev: bool,
}

/// `<repo>=<number>`, as `--pr` takes it.
fn repo_and_number(input: &str) -> std::result::Result<(String, u64), String> {
    let expected = "expected `<repo>=<number>`, e.g. `backend=12`";
    let (repo, number) = repo_and_value(input).ok_or(expected)?;
    let number = pull_request_number(number).ok_or(expected)?;
    Ok((repo.to_string(), number))
}

/// `<repo>=<branch>`, as `--branch` takes it.
fn repo_and_branch(input: &str) -> std::result::Result<(String, String), String> {
    let (repo, branch) =
        repo_and_value(input).ok_or("expected `<repo>=<branch>`, e.g. `agent=fix/prompt`")?;
    Ok((repo.to_string(), branch.to_string()))
}

fn repo_and_value(input: &str) -> Option<(&str, &str)> {
    input
        .split_once('=')
        .filter(|(repo, value)| !repo.is_empty() && !value.is_empty())
}

/// `12` or `#12`.
fn pull_request_number(text: &str) -> Option<u64> {
    text.strip_prefix('#').unwrap_or(text).parse().ok()
}

#[derive(Subcommand)]
enum RepoCommand {
    /// Register a git repo in the project and clone it
    Add {
        url: String,
        /// Key for the repo, also its folder name (defaults to the url's last path segment)
        #[arg(long)]
        name: Option<String>,
        /// Short name accepted wherever the key is, e.g. `frontend`
        #[arg(long)]
        alias: Option<String>,
        /// Base branch for this repo, overriding the project's
        #[arg(long)]
        base: Option<String>,
        /// Machine where this repo lives, overriding the project's: `local`, `<host>` or `<host>:<path>`
        #[arg(long)]
        location: Option<String>,
    },
    /// List the project's repos
    Ls,
    /// Change the alias, the base branch, the url, the location or the commands of a repo
    Set {
        /// Key or alias of the repo
        repo: String,
        #[command(flatten)]
        changes: RepoSetArgs,
    },
    /// Unregister a repo, given by key or alias, and delete its clone
    Rm { key: String },
}

#[derive(Args)]
#[group(required = true, multiple = true)]
struct ProjectSetArgs {
    /// Default base branch for the project's repos
    #[arg(long)]
    base: Option<String>,
    /// Machine where the project's repos live: `local`, `<host>` or `<host>:<path>`
    #[arg(long, conflicts_with = "no_location")]
    location: Option<String>,
    /// Go back to this machine
    #[arg(long)]
    no_location: bool,
}

#[derive(Args)]
#[group(required = true, multiple = true)]
struct RepoSetArgs {
    /// Short name accepted wherever the key is, e.g. `frontend`
    #[arg(long, conflicts_with = "no_alias")]
    alias: Option<String>,
    /// Remove the alias
    #[arg(long)]
    no_alias: bool,
    /// Base branch for this repo, overriding the project's
    #[arg(long, conflicts_with = "no_base")]
    base: Option<String>,
    /// Go back to the project's base branch
    #[arg(long)]
    no_base: bool,
    /// New git url: the clone is fetched from it
    #[arg(long)]
    url: Option<String>,
    /// Machine where this repo lives, overriding the project's: `local`, `<host>` or `<host>:<path>`
    #[arg(long, conflicts_with = "no_location")]
    location: Option<String>,
    /// Go back to the project's location
    #[arg(long)]
    no_location: bool,
    /// Shell command that `octok up` runs in a checkout before its first run, e.g. `npm ci`
    #[arg(long, conflicts_with = "no_setup")]
    setup: Option<String>,
    /// Remove the setup command
    #[arg(long)]
    no_setup: bool,
    /// Shell command that `octok up` runs in the checkout to start the repo
    #[arg(long, group = "run_change")]
    run: Option<String>,
    /// Remove the run command
    #[arg(long, group = "run_change")]
    no_run: bool,
    /// Change the run command in the workspace of this branch only
    #[arg(
        long,
        requires = "run_change",
        conflicts_with_all = [
            "alias", "no_alias", "base", "no_base", "url", "location", "no_location", "setup",
            "no_setup",
        ]
    )]
    branch: Option<String>,
}

impl RepoSetArgs {
    /// `location` is `--location` once resolved.
    fn changes<'a>(&'a self, location: Option<&'a Location>) -> RepoChanges<'a> {
        RepoChanges {
            alias: optional_change(self.alias.as_deref(), self.no_alias),
            base: optional_change(self.base.as_deref(), self.no_base),
            url: self.url.as_deref(),
            location: optional_change(location, self.no_location),
            setup: optional_change(self.setup.as_deref(), self.no_setup),
            run: optional_change(self.run.as_deref(), self.no_run),
        }
    }
}

/// `--x <value>` sets, `--no-x` clears, neither leaves the setting alone.
fn optional_change<T>(value: Option<T>, clear: bool) -> Option<Option<T>> {
    match (value, clear) {
        (Some(value), _) => Some(Some(value)),
        (None, true) => Some(None),
        (None, false) => None,
    }
}

fn main() -> ExitCode {
    end_quietly_when_the_reader_goes_away();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(refusal) => return refuse_command_line(&refusal),
    };
    let out = Output::new(cli.json);
    if cli.json {
        Ssh::never_ask();
    }
    run(cli, out).unwrap_or_else(|error| out.failure(&error))
}

/// Rust ignores SIGPIPE, so printing to a pipe that was closed panics. With the default
/// action octok ends as any command does in `octok status | head -1`.
fn end_quietly_when_the_reader_goes_away() {
    // SAFETY: the first thing `main` does, before any thread exists, and restoring the
    // default action of a signal has no other effect on the process.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

/// clap refuses a command line before `--json` is read, so the arguments are searched
/// for it. Help and the version are not failures: they stay text.
fn refuse_command_line(refusal: &clap::Error) -> ExitCode {
    let wants_json = std::env::args()
        .skip(1)
        .take_while(|argument| argument != "--")
        .any(|argument| argument == "--json" || argument.starts_with("--json="));
    if wants_json && refusal.use_stderr() {
        print_usage_failure(&refusal.render().to_string())
    } else {
        refusal.exit()
    }
}

fn run(cli: Cli, out: Output) -> Result<ExitCode> {
    let registry = Registry::new(Home::from_env()?);
    let explicit_project = cli.project.as_deref();
    let resolve_project = || registry.resolve_project(explicit_project);
    match cli.command {
        Command::Project(ProjectCommand::Set { changes }) => {
            return set_project(&registry, &resolve_project()?, &changes, out);
        }
        Command::SetSshPassphrase { host } => {
            out.needs_terminal("octok set-ssh-passphrase")?;
            set_ssh_passphrase(&registry, host, resolve_project)?;
        }
        Command::Project(cmd) => project(&registry, cmd, out)?,
        Command::Repo(cmd) => repo(&registry, &resolve_project()?, cmd, out)?,
        Command::Fetch => return fetch(&registry, &resolve_project()?, out),
        Command::Branch {
            name: branch,
            location,
            allow_stale_dev,
        } => {
            let workspace = WorkspaceRequest {
                branch: &branch,
                location: location.as_deref(),
                allow_stale_dev,
                open: create_workspace,
            };
            open(&registry, &resolve_project()?, &workspace, out)?
        }
        Command::Checkout {
            create,
            branch,
            location,
            allow_stale_dev,
        } => {
            let workspace = WorkspaceRequest {
                branch: &branch,
                location: location.as_deref(),
                allow_stale_dev,
                open: if create {
                    create_workspace
                } else {
                    checkout_workspace
                },
            };
            open(&registry, &resolve_project()?, &workspace, out)?
        }
        Command::Rm { force, branch } => {
            let project_name = resolve_project()?;
            let name = remove_workspace(&registry, &project_name, &branch, force)?;
            let document = json!({"project": project_name, "removed": name});
            out.result(&document, |_| {
                println!("removed workspace `{name}` from {project_name}");
            });
        }
        Command::Pr(PrCommand::Review(args)) => review(&registry, &resolve_project()?, &args, out)?,
        Command::Pull { branch } => {
            return pull(&registry, explicit_project, branch.as_deref(), out);
        }
        Command::Dev(DevCommand::Save {
            repos,
            files,
            branch,
        }) => {
            let usage = "octok dev save --branch <branch>";
            let (project_name, workspace) =
                target_workspace(&registry, explicit_project, branch.as_deref(), usage)?;
            let project = registry.load_project(&project_name)?;
            let (repos, files) = (as_strs(&repos), as_strs(&files));
            let saved = save_dev_changes(registry.home(), &project, &workspace, &repos, &files)?;
            print_dev_changes(out, &project, &saved, false);
        }
        Command::Dev(DevCommand::Show { repos }) => {
            let project = registry.load_project(&resolve_project()?)?;
            let saved = dev_changes(registry.home(), &project, &as_strs(&repos))?;
            print_dev_changes(out, &project, &saved, true);
        }
        Command::Up { repos, branch } => {
            return up(&registry, explicit_project, branch.as_deref(), &repos, out);
        }
        Command::Status => {
            let project = registry.load_project(&resolve_project()?)?;
            out.result(&status(registry.home(), &project)?, print_status);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn project(registry: &Registry, cmd: ProjectCommand, out: Output) -> Result<()> {
    match cmd {
        ProjectCommand::Init {
            name,
            base,
            location,
        } => {
            refuse_unusable_project_name(registry, &name)?;
            let mut location = resolve_optional_location(location.as_deref())?;
            let mut base = base;
            if out.may_ask() {
                if base.is_none() {
                    base = ask_project_base(&name)?;
                }
                if location.is_none() {
                    location = ask_location(LOCAL)?;
                }
            }
            let base = base.unwrap_or_else(|| DEFAULT_BASE.to_string());
            registry.init_project(&name, &base)?;
            out.note(format_args!(
                "created project `{name}` (base: {base}) at {}",
                registry.home().project_file(&name).display()
            ));
            if let Some(location) = location {
                out.note(format_args!("its repos live at {location}"));
                registry.update_project(&name, |project| {
                    project.set_location(Some(location));
                    Ok(())
                })?;
            }
            out.result(&ProjectDocument::read(registry, &name)?, |_| {
                println!("`{name}` is now the default project");
            });
        }
        ProjectCommand::Ls => {
            let projects = registry.list_projects()?;
            let default = registry.default_project()?;
            print_projects(out, &projects, default.as_deref());
        }
        ProjectCommand::Use { name } => {
            registry.set_default_project(&name)?;
            out.result(&json!({"default": name}), |_| {
                println!("default project: {name}");
            });
        }
        ProjectCommand::Set { .. } => unreachable!("handled by `run`, which knows the project"),
        ProjectCommand::Rm { force, name } => {
            remove_project(registry, &name, force)?;
            let default = registry.default_project()?;
            out.result(&json!({"removed": name, "default": default}), |_| {
                println!("removed project `{name}`");
                if default.is_none() {
                    println!("no default project: choose one with `octok project use <name>`");
                }
            });
        }
    }
    Ok(())
}

fn repo(registry: &Registry, project_name: &str, cmd: RepoCommand, out: Output) -> Result<()> {
    match cmd {
        RepoCommand::Add {
            url,
            name,
            alias,
            base,
            location,
        } => {
            let mut settings = RepoSettings {
                key: name,
                alias,
                base,
                location: resolve_optional_location(location.as_deref())?,
            };
            let may_ask = out.may_ask();
            let project = registry.load_project(project_name)?;
            if may_ask {
                ask_repo_settings(&project, &url, &mut settings)?;
            }
            // Asked once the repo is cloned, when its branches and their dates are known.
            let choose_base = |_key: &str, branches: &OriginBranches| {
                may_ask.then(|| ask_repo_base(&project, branches)).flatten()
            };
            let repo = NewRepo {
                url: &url,
                key: settings.key.as_deref(),
                base: settings.base.as_deref(),
                alias: settings.alias.as_deref(),
                location: settings.location.as_ref(),
            };
            let key = register_repo(
                registry,
                project_name,
                &repo,
                confirm_missing_base(out),
                choose_base,
            )?;
            let project = registry.load_project(project_name)?;
            out.result(&RepoDocument::new(&project, None, &key), |_| {
                let label = repo_label(&key, settings.alias.as_deref());
                println!("added `{label}` to {project_name}: {url}");
                let location = project.effective_location(&project.repos[&key], None);
                if *location != Location::Local {
                    println!("cloned at {location}");
                }
            });
        }
        RepoCommand::Ls => print_repos(out, &registry.load_project(project_name)?),
        RepoCommand::Set { repo, changes } => {
            if let Some(branch) = &changes.branch {
                let run = changes.run.as_deref();
                let key = set_workspace_run(registry, project_name, &repo, branch, run)?;
                let workspace = workspace_name(branch);
                let project = registry.load_project(project_name)?;
                let document = RepoDocument::new(&project, Some(&workspace), &key);
                out.result(&document, |_| {
                    println!("updated `{key}` in workspace `{workspace}` of {project_name}");
                });
                return Ok(());
            }
            let location = resolve_optional_location(changes.location.as_deref())?;
            let key = update_repo(
                registry,
                project_name,
                &repo,
                &changes.changes(location.as_ref()),
                confirm_missing_base(out),
            )?;
            let project = registry.load_project(project_name)?;
            out.result(&RepoDocument::new(&project, None, &key), |_| {
                let label = repo_label(&key, project.repos[&key].alias.as_deref());
                println!("updated `{label}` in {project_name}");
            });
        }
        RepoCommand::Rm { key } => {
            let key = unregister_repo(registry, project_name, &key)?;
            out.result(&json!({"project": project_name, "removed": key}), |_| {
                println!("removed `{key}` from {project_name}");
            });
        }
    }
    Ok(())
}

fn set_project(
    registry: &Registry,
    project_name: &str,
    changes: &ProjectSetArgs,
    out: Output,
) -> Result<ExitCode> {
    if let Some(base) = &changes.base {
        set_project_base(registry, project_name, base)?;
        out.note(format_args!("base of `{project_name}` is now `{base}`"));
    }
    let mut cloned_again = None;
    if changes.location.is_some() || changes.no_location {
        let location = resolve_optional_location(changes.location.as_deref())?;
        let shown = location.clone().unwrap_or(Location::Local);
        set_project_location(registry, project_name, location)?;
        out.note(format_args!(
            "repos of `{project_name}` now live at {shown}"
        ));
        // The caches of the repos that moved are gone: clone them where they live now.
        let project = registry.load_project(project_name)?;
        cloned_again = Some(fetch_repos(registry.home(), &project));
    }
    let mut document = ProjectDocument::read(registry, project_name)?;
    document.repos = cloned_again;
    let report = document.repos.as_deref().unwrap_or_default();
    let project = registry.load_project(project_name)?;
    out.result(&document, |_| print_fetch_report(&project, report));
    Ok(report_exit_code(report))
}

/// Warns on stderr about the repos whose remote lacks the new base.
fn set_project_base(registry: &Registry, project_name: &str, base: &str) -> Result<()> {
    let project = registry.update_project(project_name, |project| {
        project.set_base(base)?;
        Ok(project.clone())
    })?;
    // From the last fetch, without the network: enough for a hint.
    let without_base: Vec<String> = status(registry.home(), &project)?
        .repos
        .into_iter()
        .filter(|repo| repo.cached && repo.commit.is_none())
        .map(|repo| repo.key)
        .collect();
    if !without_base.is_empty() {
        eprintln!(
            "warning: branch `{base}` is not on the remote of: {}",
            without_base.join(", ")
        );
    }
    Ok(())
}

fn fetch(registry: &Registry, project_name: &str, out: Output) -> Result<ExitCode> {
    let project = registry.load_project(project_name)?;
    let report = Report {
        project: &project.name,
        workspace: None,
        repos: fetch_repos(registry.home(), &project),
    };
    out.result(&report, |report| {
        print_fetch_report(&project, &report.repos)
    });
    Ok(report_exit_code(&report.repos))
}

/// What `octok branch` and `octok checkout` ask for.
struct WorkspaceRequest<'a> {
    branch: &'a str,
    location: Option<&'a str>,
    allow_stale_dev: bool,
    open: OpenWorkspace,
}

/// `create_workspace` for a new branch, `checkout_workspace` for one that exists.
type OpenWorkspace = fn(&Registry, &str, &str, Option<&Location>, bool) -> Result<NewWorkspace>;

fn open(
    registry: &Registry,
    project_name: &str,
    request: &WorkspaceRequest,
    out: Output,
) -> Result<()> {
    let location = resolve_optional_location(request.location)?;
    let workspace = (request.open)(
        registry,
        project_name,
        request.branch,
        location.as_ref(),
        request.allow_stale_dev,
    )?;
    let name = workspace_name(request.branch);
    print_workspace(out, project_name, &name, &workspace);
    Ok(())
}

/// What to review in one repo, as asked.
enum Pick<'a> {
    PullRequest(u64),
    Branch(&'a str),
}

/// A branch to review in one repo, and the pull request it comes from, if any.
struct Reviewed {
    repo: String,
    branch: String,
    pull_request: Option<u64>,
}

fn review(registry: &Registry, project_name: &str, args: &ReviewArgs, out: Output) -> Result<()> {
    let project = registry.load_project(project_name)?;
    if project.repos.is_empty() {
        return Err(Error::NoRepos(project.name));
    }
    if let Some(name) = &args.name {
        // A workspace is named after a branch, so the name must be one.
        Project::new(&project.name, name)?;
    }
    let gh = Gh::new(registry.home().root());
    let picks = picks_from_flags(&project, args)?;
    let reviewed = if !picks.is_empty() {
        picks
            .into_iter()
            .map(|(key, pick)| reviewed_pick(&gh, &project, key, &pick))
            .collect::<Result<_>>()?
    } else if out.may_ask() {
        reviewed_from_terminal(registry.home(), &project, &gh)?
    } else {
        Vec::new()
    };
    let Some(branch) = args
        .name
        .clone()
        .or_else(|| review_name(&project, &reviewed))
    else {
        return Err(Error::NothingToReview);
    };
    let chosen: HashMap<String, String> = reviewed
        .into_iter()
        .map(|reviewed| (reviewed.repo, reviewed.branch))
        .collect();
    let location = resolve_optional_location(args.location.as_deref())?;
    let workspace = review_workspace(
        registry,
        project_name,
        &branch,
        &chosen,
        location.as_ref(),
        args.allow_stale_dev,
    )?;
    print_workspace(out, project_name, &workspace_name(&branch), &workspace);
    Ok(())
}

/// What `--pr` and `--branch` ask for, by repo key, in the order of the repos of the
/// project. Checked before `gh` is called for any of them.
fn picks_from_flags<'a>(
    project: &'a Project,
    args: &'a ReviewArgs,
) -> Result<Vec<(&'a str, Pick<'a>)>> {
    let pull_requests = args
        .pull_requests
        .iter()
        .map(|(repo, number)| (repo, Pick::PullRequest(*number)));
    let branches = args
        .branches
        .iter()
        .map(|(repo, branch)| (repo, Pick::Branch(branch)));
    let mut picks: HashMap<&str, Pick> = HashMap::new();
    for (repo, pick) in pull_requests.chain(branches) {
        let key = project.resolve_repo(repo)?;
        if picks.insert(key, pick).is_some() {
            return Err(Error::RepoChosenTwice(key.to_string()));
        }
    }
    let in_project_order = project
        .repos
        .keys()
        .filter_map(|key| picks.remove_entry(key.as_str()))
        .collect();
    Ok(in_project_order)
}

/// Asks, for each repo, which pull request or branch to review. A repo with an empty
/// answer stays on its base. An answer that cannot be reviewed is asked again.
fn reviewed_from_terminal(home: &Home, project: &Project, gh: &Gh) -> Result<Vec<Reviewed>> {
    let mut reviewed = Vec::new();
    for (key, repo) in &project.repos {
        let label = repo_label(key, repo.alias.as_deref());
        let open = gh
            .open_pull_requests(&repo.url)
            .inspect_err(|error| eprintln!("cannot list the pull requests of {label}:\n{error}"))
            .ok();
        let base = project.effective_base(repo);
        let answer = ask_what_to_review(&label, base, open.as_deref(), |answer| {
            let pick = match pull_request_number(answer) {
                Some(number) => Pick::PullRequest(number),
                None => Pick::Branch(answer),
            };
            let reviewed = reviewed_pick(gh, project, key, &pick)?;
            check_remote_branch(home, project, key, &reviewed.branch)?;
            Ok(reviewed)
        })?;
        reviewed.extend(answer);
    }
    Ok(reviewed)
}

/// The branch that `pick` means for the repo `key`. Reads a pull request with `gh`.
fn reviewed_pick(gh: &Gh, project: &Project, key: &str, pick: &Pick) -> Result<Reviewed> {
    let repo = &project.repos[key];
    let (branch, pull_request) = match *pick {
        Pick::Branch(branch) => (branch.to_string(), None),
        Pick::PullRequest(number) => {
            let pull_request = gh.pull_request(&repo.url, number)?;
            if pull_request.from_fork {
                return Err(Error::ForkPullRequest {
                    repo: key.to_string(),
                    number,
                });
            }
            eprintln!(
                "{}: pull request #{number} is the branch `{}`",
                repo_label(key, repo.alias.as_deref()),
                pull_request.head_branch
            );
            (pull_request.head_branch, Some(number))
        }
    };
    Ok(Reviewed {
        repo: key.to_string(),
        branch,
        pull_request,
    })
}

/// `pr-<repo>-<number>` of the first pull request, else the first branch.
/// `None` when nothing is reviewed.
fn review_name(project: &Project, reviewed: &[Reviewed]) -> Option<String> {
    let of_pull_request = reviewed.iter().find_map(|reviewed| {
        let number = reviewed.pull_request?;
        let repo = &project.repos[&reviewed.repo];
        let repo_name = repo.alias.as_deref().unwrap_or(&reviewed.repo);
        Some(format!("pr-{repo_name}-{number}"))
    });
    of_pull_request.or_else(|| reviewed.first().map(|reviewed| reviewed.branch.clone()))
}

fn pull(
    registry: &Registry,
    explicit_project: Option<&str>,
    branch: Option<&str>,
    out: Output,
) -> Result<ExitCode> {
    let (project_name, workspace) =
        target_workspace(registry, explicit_project, branch, "octok pull <branch>")?;
    let project = registry.load_project(&project_name)?;
    let report = Report {
        project: &project.name,
        workspace: Some(&workspace),
        repos: pull_workspace(registry.home(), &project, &workspace)?,
    };
    out.result(&report, |report| print_pull_report(&project, &report.repos));
    Ok(report_exit_code(&report.repos))
}

/// Stays attached until every command has ended. Ctrl-C stops them all.
///
/// With `--json` every event is a document on its own line, after one that says what started.
fn up(
    registry: &Registry,
    explicit_project: Option<&str>,
    branch: Option<&str>,
    repos: &[String],
    out: Output,
) -> Result<ExitCode> {
    let usage = "octok up --branch <branch>";
    let (project_name, workspace) = target_workspace(registry, explicit_project, branch, usage)?;
    let project = registry.load_project(&project_name)?;
    let running = start_workspace(registry.home(), &project, &workspace, &as_strs(repos))?;
    let interrupted = stop_on_signal(running.stopper());
    let labels = up_labels(&project, running.started());
    let started = json!({
        "event": "started",
        "project": project_name,
        "workspace": workspace,
        "repos": running.started().collect::<Vec<_>>(),
    });
    out.result(&started, |_| {});
    let all_succeeded = running.wait(|event| {
        out.result(&event, |event| {
            print_up_event(&labels, *event, interrupted.load(Ordering::SeqCst));
        });
    });
    Ok(if interrupted.load(Ordering::SeqCst) {
        ExitCode::from(INTERRUPTED)
    } else if all_succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Asks for a stop at Ctrl-C, and when octok is told to end or loses its terminal.
/// Returns the flag that says whether that happened.
fn stop_on_signal(stopper: Stopper) -> Arc<AtomicBool> {
    let interrupted = Arc::new(AtomicBool::new(false));
    let mut signals =
        Signals::new([SIGINT, SIGTERM, SIGHUP]).expect("signal handlers can be installed");
    let flag = Arc::clone(&interrupted);
    thread::spawn(move || {
        for _signal in signals.forever() {
            let first = !flag.swap(true, Ordering::SeqCst);
            stopper.stop();
            if first {
                // The terminal may be the thing that just went away.
                let _ = writeln!(io::stderr(), "stopping...");
            }
        }
    });
    interrupted
}

/// Project and workspace a command acts on: the ones of `branch`, else the ones that
/// contain the current directory. `usage` shows how to pass the branch.
fn target_workspace(
    registry: &Registry,
    explicit_project: Option<&str>,
    branch: Option<&str>,
    usage: &'static str,
) -> Result<(String, String)> {
    match branch {
        Some(branch) => Ok((
            registry.resolve_project(explicit_project)?,
            workspace_name(branch),
        )),
        None => std::env::current_dir()
            .ok()
            .and_then(|dir| locate_workspace(registry.home(), &dir))
            .ok_or(Error::NotInWorkspace(usage)),
    }
}

fn as_strs(values: &[String]) -> Vec<&str> {
    values.iter().map(String::as_str).collect()
}

/// `--location` connects to the machine once, to find the octok home there.
/// Fails before any question is asked and before another machine is reached.
fn refuse_unusable_project_name(registry: &Registry, name: &str) -> Result<()> {
    Project::new(name, DEFAULT_BASE)?;
    if registry.list_projects()?.iter().any(|known| known == name) {
        return Err(Error::ProjectExists(name.to_string()));
    }
    Ok(())
}

fn resolve_optional_location(input: Option<&str>) -> Result<Option<Location>> {
    input.map(resolve_location).transpose()
}

/// The other machines of a project, each one once.
fn remote_hosts(project: &Project) -> Vec<String> {
    let mut hosts: Vec<String> = Vec::new();
    for location in project.locations() {
        if let Some(host) = location.host()
            && !hosts.iter().any(|known| known == host)
        {
            hosts.push(host.to_string());
        }
    }
    hosts
}

/// Unlocks the key of `host`, or of every other machine of the project.
fn set_ssh_passphrase(
    registry: &Registry,
    host: Option<String>,
    resolve_project: impl FnOnce() -> Result<String>,
) -> Result<()> {
    let hosts = match host {
        Some(host) => vec![host],
        None => {
            let project = registry.load_project(&resolve_project()?)?;
            let hosts = remote_hosts(&project);
            if hosts.is_empty() {
                println!("project `{}` has no repo on another machine", project.name);
            }
            hosts
        }
    };
    remember_passphrases(&hosts)
}

/// `ssh-add` asks for each passphrase at the terminal: octok never sees it.
fn remember_passphrases(hosts: &[String]) -> Result<()> {
    for host in hosts {
        let keys = Ssh::new(host)?.remember_passphrase()?;
        if keys.is_empty() {
            println!("{host}: ssh uses no key file for this machine");
        }
        for key in keys {
            println!("{host}: added {} to the ssh agent", key.display());
        }
    }
    Ok(())
}

/// Asks only a person at a terminal: scripts and agents get the error instead of a
/// question nobody answers.
fn confirm_missing_base(out: Output) -> impl FnOnce(&str, &str) -> bool {
    move |repo, base| {
        out.may_ask()
            && confirm(
                &format!(
                    "branch `{base}` is not on the remote of `{repo}`.\nUse it as base anyway?"
                ),
                io::stdin().lock(),
                io::stderr(),
            )
    }
}
