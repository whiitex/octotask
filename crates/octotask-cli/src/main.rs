mod utils;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use octotask_core::{Home, Registry, Result};

use utils::{print_projects, print_repos};

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
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Create a new project; the first one becomes the default
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
}

#[derive(Subcommand)]
enum RepoCommand {
    /// Register a git repo in the project
    Add {
        url: String,
        /// Key for the repo (defaults to the url's last path segment)
        #[arg(long)]
        name: Option<String>,
        /// Base branch for this repo, overriding the project's
        #[arg(long)]
        base: Option<String>,
    },
    /// List the project's repos
    Ls {
        #[arg(long)]
        json: bool,
    },
    /// Unregister a repo
    Rm { key: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<()> {
    let registry = Registry::new(Home::from_env()?);
    match cli.command {
        Command::Project(cmd) => project(&registry, cmd),
        Command::Repo(cmd) => {
            let name = registry.resolve_project(cli.project.as_deref())?;
            repo(&registry, &name, cmd)
        }
    }
}

fn project(registry: &Registry, cmd: ProjectCommand) -> Result<()> {
    match cmd {
        ProjectCommand::Init { name, base } => {
            registry.init_project(&name, &base)?;
            println!(
                "created project `{name}` (base: {base}) at {}",
                registry.home().project_file(&name).display()
            );
            if registry.default_project()?.as_deref() == Some(name.as_str()) {
                println!("`{name}` is now the default project");
            }
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
    }
    Ok(())
}

fn repo(registry: &Registry, project_name: &str, cmd: RepoCommand) -> Result<()> {
    let mut project = registry.load_project(project_name)?;
    match cmd {
        RepoCommand::Add { url, name, base } => {
            let key = project.add_repo(&url, name.as_deref(), base.as_deref())?;
            registry.save_project(&project)?;
            println!("added `{key}` to {project_name}: {url}");
        }
        RepoCommand::Ls { json } => print_repos(&project, json),
        RepoCommand::Rm { key } => {
            project.remove_repo(&key)?;
            registry.save_project(&project)?;
            println!("removed `{key}` from {project_name}");
        }
    }
    Ok(())
}
