mod utils;

use std::io::{self, IsTerminal};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use octotask_core::{
    Error, Home, NewRepo, Registry, Result, create_workspace, fetch_repos, locate_workspace,
    pull_workspace, register_repo, remove_project, remove_workspace, status, unregister_repo,
    workspace_name, workspace_path,
};

use utils::{
    confirm, print_projects, print_pull_report, print_report, print_repos, print_status, repo_label,
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
    Branch { name: String },
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
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Create a new project and make it the default
    Init {
        name: String,
        /// Default base branch for the project's repos
        #[arg(long, default_value = "main")]
        base: String,
    },
    /// List projects
    Ls {
        #[arg(long)]
        json: bool,
    },
    /// Set the default project
    Use { name: String },
    /// Delete a project, its list of repos and their clones
    Rm { name: String },
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
    },
    /// List the project's repos
    Ls {
        #[arg(long)]
        json: bool,
    },
    /// Unregister a repo, given by key or alias, and delete its clone
    Rm { key: String },
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
        Command::Project(cmd) => project(&registry, cmd)?,
        Command::Repo(cmd) => repo(&registry, &resolve_project()?, cmd)?,
        Command::Fetch => return fetch(&registry, &resolve_project()?),
        Command::Branch { name } => create(&registry, &resolve_project()?, &name)?,
        Command::Checkout {
            create: true,
            branch,
        } => create(&registry, &resolve_project()?, &branch)?,
        Command::Checkout {
            create: false,
            branch,
        } => {
            let path = workspace_path(registry.home(), &resolve_project()?, &branch)?;
            println!("{}", path.display());
        }
        Command::Rm { force, branch } => {
            let project_name = resolve_project()?;
            let project = registry.load_project(&project_name)?;
            let name = remove_workspace(registry.home(), &project, &branch, force)?;
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
        ProjectCommand::Init { name, base } => {
            registry.init_project(&name, &base)?;
            println!(
                "created project `{name}` (base: {base}) at {}",
                registry.home().project_file(&name).display()
            );
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
        ProjectCommand::Rm { name } => {
            remove_project(registry, &name)?;
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
        } => {
            let repo = NewRepo {
                url: &url,
                key: name.as_deref(),
                base: base.as_deref(),
                alias: alias.as_deref(),
            };
            let key = register_repo(registry, project_name, &repo, confirm_missing_base)?;
            let label = repo_label(&key, alias.as_deref());
            println!("added `{label}` to {project_name}: {url}");
        }
        RepoCommand::Ls { json } => print_repos(&registry.load_project(project_name)?, json),
        RepoCommand::Rm { key } => {
            let key = unregister_repo(registry, project_name, &key)?;
            println!("removed `{key}` from {project_name}");
        }
    }
    Ok(())
}

fn fetch(registry: &Registry, project_name: &str) -> Result<ExitCode> {
    let project = registry.load_project(project_name)?;
    let report = fetch_repos(registry.home(), &project);
    Ok(print_report(&project, report, |()| "fetched".to_string()))
}

/// Prints only the path on stdout, so `cd "$(octok branch <name>)"` works.
fn create(registry: &Registry, project_name: &str, branch: &str) -> Result<()> {
    let project = registry.load_project(project_name)?;
    let workspace = create_workspace(registry.home(), &project, branch)?;
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
