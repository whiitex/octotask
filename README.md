# Octotask

`octok` runs the same git operation on every repository of a project.

A feature often touches several repositories at once: a frontend, a backend, an agent.
Working on it means creating the same branch in each of them, keeping them up to date and remembering where each one stands.
`octok` groups those repositories into a **project** and gives each feature a **workspace**: one folder with a checkout of every repository, all on the same branch.

- **One command for all repos.** `octok branch feat/login` creates the branch in every repository, starting from a freshly fetched base branch.
- **Isolated workspaces.** Each workspace is a set of `git worktree`s, so several features can be open side by side without stashing or switching branches.
- **No repeated downloads.** Each repository is cloned once into a cache, and every workspace is created from it in an instant.
- **Plain git underneath.** `octok` calls your own `git`, so your credentials, SSH config and hooks apply, and every checkout is a normal git working tree.

```console
$ octok project init shop
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
$ octok repo add git@github.com:acme/shop-backend.git --alias backend

$ cd "$(octok branch feat/login)"
$ ls
shop-backend  shop-frontend

$ octok status
project: shop

repos:
  shop-frontend (frontend)  main  4f2a91c  Add cart page (2 days ago)
  shop-backend (backend)    main  a81d0e3  Fix order totals (5 hours ago)

workspaces:
  feat-login  /Users/you/.octotask/workspaces/shop/feat-login
    shop-frontend (frontend)  feat/login  4f2a91c  Add cart page (2 days ago)
    shop-backend (backend)    feat/login  a81d0e3  Fix order totals (5 hours ago)
```

## Installation

Requirements:

- [Rust](https://rustup.rs) 1.89 or newer, to build the binary.
- `git` 2.37 or newer.
- macOS or Linux.

Clone the repository and install the `octok` binary into `~/.cargo/bin`:

```sh
git clone https://github.com/whiitex/octotask.git
cd octotask
make install
```

`make install` runs `cargo install --path crates/octotask-cli --locked`.
Check the installation with `octok --version`.

`octok` keeps everything it manages under `~/.octotask`.
Set the `OCTOTASK_HOME` environment variable to use another folder.

| Path | Content |
|---|---|
| `~/.octotask/config.yaml` | The default project. |
| `~/.octotask/projects/<project>.yaml` | The repositories of a project. |
| `~/.octotask/cache/<project>/<repo>.git` | The bare clone of a repository. |
| `~/.octotask/workspaces/<project>/<workspace>/<repo>` | The checkouts of a workspace. |

## Features

Every command acts on the default project.
Pass `-p <project>` to act on another one, for example `octok -p shop status`.

Commands that list data also accept `--json`, for scripts and agents.
Every JSON output carries `"schema": 1`.

### Projects: `octok project`

A project is a named group of repositories with a default base branch.

| Command | What it does |
|---|---|
| `octok project init <name> [--base <branch>]` | Creates a project and makes it the default. The base branch defaults to `main`. |
| `octok project ls [--json]` | Lists the projects and marks the default one with `*`. |
| `octok project use <name>` | Sets the default project. |
| `octok project set --base <branch>` | Changes the base branch of the project, and warns about the repositories that do not have it. |
| `octok project rm <name>` | Deletes a project and the clones of its repositories. Refused while the project has workspaces. |

### Repositories: `octok repo`

Each repository is stored under a **key**, which is also the name of its folder in every workspace.
The key defaults to the repository name in the URL: `git@github.com:acme/shop-frontend.git` gives `shop-frontend`.
A repository can also have a shorter **alias**, accepted wherever the key is.

| Command | What it does |
|---|---|
| `octok repo add <url> [--name <key>] [--alias <alias>] [--base <branch>]` | Registers a repository and clones it into the cache. `--name` replaces the key, which is needed when two repositories have the same name. `--base` overrides the base branch of the project for this repository. |
| `octok repo ls [--json]` | Lists the repositories with their base branch and URL. |
| `octok repo set <repo> --alias <alias>` | Sets or changes the alias. `--no-alias` removes it. |
| `octok repo set <repo> --base <branch>` | Changes the base branch. `--no-base` goes back to the base branch of the project. |
| `octok repo set <repo> --url <url>` | Changes the URL and fetches the clone from it. |
| `octok repo rm <repo>` | Unregisters a repository and deletes its clone. Refused while a workspace uses it. |

The options of `octok repo set` can be combined in one command.

When a base branch is not on the remote, `octok` asks for confirmation before using it.
Without a terminal, for example in a script, it refuses instead.

### Workspaces: `octok branch`, `octok checkout`, `octok rm`

A workspace is a folder with one checkout per repository, all on the same branch.
It is named after its branch, with `/` replaced by `-`: the branch `feat/login` lives in the workspace `feat-login`.

| Command | What it does |
|---|---|
| `octok branch <branch>` | Fetches every repository, creates `<branch>` from its up-to-date base branch and checks it out in a new workspace. Prints the path of the workspace. |
| `octok checkout -b <branch>` | Same as `octok branch <branch>`. |
| `octok checkout <branch>` | Prints the path of an existing workspace. |
| `octok rm <branch> [--force]` | Deletes a workspace: its checkouts, its folder and its local branches. |

A program cannot change the folder of the shell that started it, so enter a workspace with `cd`:

```sh
cd "$(octok branch feat/login)"     # create and enter
cd "$(octok checkout feat/login)"   # enter an existing one
```

**Publishing a branch.**
The new branch exists only on your machine.
A plain `git push` inside a checkout publishes it and sets its upstream, with no need for `-u origin <branch>`.

**Completing a workspace.**
Run `octok branch <branch>` again on an existing workspace to add the checkouts it lacks.
This brings in a repository registered after the workspace was created, finishes a creation that was interrupted, and restores a checkout whose folder was deleted by hand, with its commits.

**Branches that already exist.**
`octok branch` refuses a branch that is already on the remote of a repository.

**Removing safely.**
`octok rm` refuses to delete a workspace that holds uncommitted changes, unpushed commits or files that do not belong to a checkout, and lists what it found.
`--force` deletes the workspace anyway and discards that work.
Files ignored by git, such as `.env`, are deleted without a warning.

### Syncing: `octok fetch`, `octok pull`

| Command | What it does |
|---|---|
| `octok fetch` | Fetches every repository of the project into the cache, and clones the ones that are missing. |
| `octok pull [<branch>]` | Fetches, then fast-forwards each checkout of a workspace to its remote branch. Without `<branch>`, it uses the workspace of the current folder. |

Both commands print one line per repository and keep going when one of them fails.
They exit with code 1 if any repository failed.

`octok pull` only fast-forwards: it never merges, rebases or resets.
A checkout whose branch has diverged from the remote, is not on the remote yet or is on a detached HEAD is skipped and reported.

### Overview: `octok status`

`octok status [--json]` shows the state of the project without touching the network:

- every repository, with its base branch and the last commit of that branch at the last fetch;
- every workspace, with the branch and the last commit of each checkout;
- the problems it finds: a repository that was never fetched, a base branch missing on the remote, a repository with no checkout in a workspace, a checkout that cannot be read, and clones left behind by repositories that are no longer in the project.

## Development

| Command | What it does |
|---|---|
| `make format` | Formats the code. |
| `make test` | Runs the tests. They need `git` and `gh`. |
| `make build` | Builds the release binary. |
| `make install` | Installs `octok` into `~/.cargo/bin`. |
| `make clean-data` | Asks for confirmation, then deletes `~/.octotask`, unpushed work in the workspaces included. |
