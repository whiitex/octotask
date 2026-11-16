mod prompts;
mod utils;

use std::io::{self, IsTerminal};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use octotask_core::{
    Error, Home, Location, NewRepo, OriginBranches, Project, Registry, RepoChanges, Result, Ssh,
    create_workspace, fetch_repos, locate_workspace, pull_workspace, register_repo, remove_project,
    remove_workspace, resolve_location, set_project_location, status, unregister_repo, update_repo,
    workspace_name, workspace_path,
};

use prompts::{
    DEFAULT_BASE, RepoSettings, ask_location, ask_project_base, ask_repo_base, ask_repo_settings,
};
use utils::{
    LOCAL, confirm, print_projects, print_pull_report, print_report, print_repos, print_status,
    repo_label,
};

/// octotask: multi-repo workspaces: one command for branches, PRs and services across repos.
#[derive(Parser)]
#[command(name = "octok", version)]
struct Cli {
    /// Project to act on (defaults to the one set with `octok project use`)
    #[arg(short, long, global = true)]
    project: Option<String>,

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
    /// interrupted creation and brings in the repos registered later.
    Branch {
        name: String,
        /// Machine where the whole workspace lives, whatever the repos and the project say:
        /// `local`, `<host>` or `<host>:<path>`
        #[arg(long)]
        location: Option<String>,
    },
    /// Delete a workspace: its worktrees, its folder and the local branches
    Rm {
        /// Also discard uncommitted changes and unpushed commits
        #[arg(long)]
        force: bool,
        branch: String,
    },
    /// Print the path of a workspace, e.g. `cd "$(octok checkout feat/login)"`
    Checkout {
        /// Create the workspace first, like `octok branch`
        #[arg(short = 'b')]
        create: bool,
        branch: String,
        /// With `-b`: machine where the whole workspace lives
        #[arg(long, requires = "create")]
        location: Option<String>,
    },
    /// Fetch, then fast-forward each worktree of a workspace to its remote branch
    Pull {
        /// Branch of the workspace (defaults to the workspace of the current directory)
        branch: Option<String>,
    },
    /// Show the project, its repos and the checkouts of every workspace
    Status {
        #[arg(long)]
        json: bool,
    },
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
    Ls {
        #[arg(long)]
        json: bool,
    },
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
    Ls {
        #[arg(long)]
        json: bool,
    },
    /// Change the alias, the base branch, the url or the location of a repo
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
}

impl RepoSetArgs {
    /// `location` is `--location` once resolved.
    fn changes<'a>(&'a self, location: Option<&'a Location>) -> RepoChanges<'a> {
        RepoChanges {
            alias: optional_change(self.alias.as_deref(), self.no_alias),
            base: optional_change(self.base.as_deref(), self.no_base),
            url: self.url.as_deref(),
            location: optional_change(location, self.no_location),
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
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let registry = Registry::new(Home::from_env()?);
    let explicit_project = cli.project.as_deref();
    let resolve_project = || registry.resolve_project(explicit_project);
    match cli.command {
        Command::Project(ProjectCommand::Set { changes }) => {
            return set_project(&registry, &resolve_project()?, &changes);
        }
        Command::SetSshPassphrase { host: Some(host) } => remember_passphrases(&[host])?,
        Command::SetSshPassphrase { host: None } => {
            let project = registry.load_project(&resolve_project()?)?;
            let hosts = remote_hosts(&project);
            if hosts.is_empty() {
                println!("project `{}` has no repo on another machine", project.name);
            }
            remember_passphrases(&hosts)?;
        }
        Command::Project(cmd) => project(&registry, cmd)?,
        Command::Repo(cmd) => repo(&registry, &resolve_project()?, cmd)?,
        Command::Fetch => return fetch(&registry, &resolve_project()?),
        Command::Branch { name, location } => {
            create(&registry, &resolve_project()?, &name, location.as_deref())?
        }
        Command::Checkout {
            create: true,
            branch,
            location,
        } => create(&registry, &resolve_project()?, &branch, location.as_deref())?,
        Command::Checkout {
            create: false,
            branch,
            ..
        } => {
            let project = registry.load_project(&resolve_project()?)?;
            let path = workspace_path(registry.home(), &project, &branch)?;
            println!("{}", path.display());
        }
        Command::Rm { force, branch } => {
            let project_name = resolve_project()?;
            let name = remove_workspace(&registry, &project_name, &branch, force)?;
            println!("removed workspace `{name}` from {project_name}");
        }
        Command::Pull { branch } => return pull(&registry, explicit_project, branch.as_deref()),
        Command::Status { json } => {
            let project = registry.load_project(&resolve_project()?)?;
            print_status(&status(registry.home(), &project)?, json);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn project(registry: &Registry, cmd: ProjectCommand) -> Result<()> {
    match cmd {
        ProjectCommand::Init {
            name,
            base,
            location,
        } => {
            refuse_unusable_project_name(registry, &name)?;
            let mut location = resolve_optional_location(location.as_deref())?;
            let mut base = base;
            if io::stdin().is_terminal() {
                if base.is_none() {
                    base = ask_project_base(&name)?;
                }
                if location.is_none() {
                    location = ask_location(LOCAL)?;
                }
            }
            let base = base.unwrap_or_else(|| DEFAULT_BASE.to_string());
            registry.init_project(&name, &base)?;
            println!(
                "created project `{name}` (base: {base}) at {}",
                registry.home().project_file(&name).display()
            );
            if let Some(location) = location {
                println!("its repos live at {location}");
                registry.update_project(&name, |project| {
                    project.set_location(Some(location));
                    Ok(())
                })?;
            }
            println!("`{name}` is now the default project");
        }
        ProjectCommand::Ls { json } => {
            let projects = registry.list_projects()?;
            let default = registry.default_project()?;
            print_projects(&projects, default.as_deref(), json);
        }
        ProjectCommand::Use { name } => {
            registry.set_default_project(&name)?;
            println!("default project: {name}");
        }
        ProjectCommand::Set { .. } => unreachable!("handled by `run`, which knows the project"),
        ProjectCommand::Rm { force, name } => {
            remove_project(registry, &name, force)?;
            println!("removed project `{name}`");
            if registry.default_project()?.is_none() {
                println!("no default project: choose one with `octok project use <name>`");
            }
        }
    }
    Ok(())
}

fn repo(registry: &Registry, project_name: &str, cmd: RepoCommand) -> Result<()> {
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
            let at_terminal = io::stdin().is_terminal();
            let project = registry.load_project(project_name)?;
            if at_terminal {
                ask_repo_settings(&project, &url, &mut settings)?;
            }
            // Asked once the repo is cloned, when its branches and their dates are known.
            let choose_base = |_key: &str, branches: &OriginBranches| {
                at_terminal
                    .then(|| ask_repo_base(&project, branches))
                    .flatten()
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
                confirm_missing_base,
                choose_base,
            )?;
            let label = repo_label(&key, settings.alias.as_deref());
            println!("added `{label}` to {project_name}: {url}");
            let project = registry.load_project(project_name)?;
            let location = project.effective_location(&project.repos[&key], None);
            if *location != Location::Local {
                println!("cloned at {location}");
            }
        }
        RepoCommand::Ls { json } => print_repos(&registry.load_project(project_name)?, json),
        RepoCommand::Set { repo, changes } => {
            let location = resolve_optional_location(changes.location.as_deref())?;
            let key = update_repo(
                registry,
                project_name,
                &repo,
                &changes.changes(location.as_ref()),
                confirm_missing_base,
            )?;
            let project = registry.load_project(project_name)?;
            let label = repo_label(&key, project.repos[&key].alias.as_deref());
            println!("updated `{label}` in {project_name}");
        }
        RepoCommand::Rm { key } => {
            let key = unregister_repo(registry, project_name, &key)?;
            println!("removed `{key}` from {project_name}");
        }
    }
    Ok(())
}

fn set_project(
    registry: &Registry,
    project_name: &str,
    changes: &ProjectSetArgs,
) -> Result<ExitCode> {
    if let Some(base) = &changes.base {
        set_project_base(registry, project_name, base)?;
    }
    if changes.location.is_none() && !changes.no_location {
        return Ok(ExitCode::SUCCESS);
    }
    let location = resolve_optional_location(changes.location.as_deref())?;
    let shown = location.clone().unwrap_or(Location::Local);
    set_project_location(registry, project_name, location)?;
    println!("repos of `{project_name}` now live at {shown}");
    // The caches of the repos that moved are gone: clone them where they live now.
    fetch(registry, project_name)
}

fn set_project_base(registry: &Registry, project_name: &str, base: &str) -> Result<()> {
    let project = registry.update_project(project_name, |project| {
        project.set_base(base)?;
        Ok(project.clone())
    })?;
    println!("base of `{project_name}` is now `{base}`");
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

fn fetch(registry: &Registry, project_name: &str) -> Result<ExitCode> {
    let project = registry.load_project(project_name)?;
    let report = fetch_repos(registry.home(), &project);
    Ok(print_report(&project, report, |()| "fetched".to_string()))
}

/// Prints only the path on stdout, so `cd "$(octok branch <name>)"` works.
fn create(
    registry: &Registry,
    project_name: &str,
    branch: &str,
    location: Option<&str>,
) -> Result<()> {
    let location = resolve_optional_location(location)?;
    let workspace = create_workspace(registry, project_name, branch, location.as_ref())?;
    let name = workspace_name(branch);
    if workspace.kept.is_empty() && workspace.restored.is_empty() {
        eprintln!("created workspace `{name}` in {project_name}");
    } else {
        eprintln!("completed workspace `{name}` in {project_name}");
    }
    if !workspace.restored.is_empty() {
        eprintln!(
            "  branch restored from the cache: {}",
            workspace.restored.join(", ")
        );
    }
    if !workspace.kept.is_empty() && !workspace.created.is_empty() {
        eprintln!("  new branch: {}", workspace.created.join(", "));
    }
    for (repo, worktree) in &workspace.remote_worktrees {
        eprintln!("  {repo}: {}", worktree.display());
    }
    println!("{}", workspace.path.display());
    Ok(())
}

fn pull(
    registry: &Registry,
    explicit_project: Option<&str>,
    branch: Option<&str>,
) -> Result<ExitCode> {
    let (project_name, workspace) = match branch {
        Some(branch) => (
            registry.resolve_project(explicit_project)?,
            workspace_name(branch),
        ),
        None => current_workspace(registry.home())?,
    };
    let project = registry.load_project(&project_name)?;
    let report = pull_workspace(registry.home(), &project, &workspace)?;
    Ok(print_pull_report(&project, report))
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

/// The workspace that contains the current directory.
fn current_workspace(home: &Home) -> Result<(String, String)> {
    std::env::current_dir()
        .ok()
        .and_then(|dir| locate_workspace(home, &dir))
        .ok_or(Error::NotInWorkspace)
}

/// Asks only at a terminal: scripts and agents get the error instead of a question nobody answers.
fn confirm_missing_base(repo: &str, base: &str) -> bool {
    let stdin = io::stdin();
    stdin.is_terminal()
        && confirm(
            &format!("branch `{base}` is not on the remote of `{repo}`.\nUse it as base anyway?"),
            stdin.lock(),
            io::stderr(),
        )
}
