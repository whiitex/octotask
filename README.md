# Octotask

`octok` runs the same git operation on every repository of a project.

A feature often touches several repositories at once: a frontend, a backend, an agent.
Working on it means creating the same branch in each of them, keeping them up to date and remembering where each one stands.
`octok` groups those repositories into a **project** and gives each feature a **workspace**: one folder with a checkout of every repository, all on the same branch.

- **One command for all repos.** `octok branch feat/login` creates the branch in every repository, starting from a freshly fetched base branch.
- **Isolated workspaces.** Each workspace is a set of `git worktree`s, so several features can be open side by side without stashing or switching branches.
- **No repeated downloads.** Each repository is cloned once into a cache, and every workspace is created from it in an instant.
- **Plain git underneath.** `octok` calls your own `git`, so your credentials, SSH config and hooks apply, and every checkout is a normal git working tree.
- **Repos on other machines.** A repository can live on another machine, such as a VM, reached over `ssh`: `octok` runs the same commands there.

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
- `ssh`, only to keep repositories on another machine. See [Locations](#locations---location).

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
| `octok project init <name> [--base <branch>] [--location <location>]` | Creates a project and makes it the default. The base branch defaults to `main`, the location to this machine. |
| `octok project ls [--json]` | Lists the projects and marks the default one with `*`. |
| `octok project use <name>` | Sets the default project. |
| `octok project set --base <branch>` | Changes the base branch of the project, and warns about the repositories that do not have it. |
| `octok project set --location <location>` | Changes the machine where the repositories of the project live. `--no-location` goes back to this machine. |
| `octok project rm <name> [--force]` | Deletes a project and the clones of its repositories, on every machine. Refused while the project has workspaces: `--force` deletes them too, and discards the work that exists only there. |

### Repositories: `octok repo`

Each repository is stored under a **key**, which is also the name of its folder in every workspace.
The key defaults to the repository name in the URL: `git@github.com:acme/shop-frontend.git` gives `shop-frontend`.
A repository can also have a shorter **alias**, accepted wherever the key is.

| Command | What it does |
|---|---|
| `octok repo add <url> [--name <key>] [--alias <alias>] [--base <branch>] [--location <location>]` | Registers a repository and clones it into the cache. `--name` replaces the key, which is needed when two repositories have the same name. `--base` and `--location` override the base branch and the location of the project for this repository. |
| `octok repo ls [--json]` | Lists the repositories with their base branch and URL, and their location when one of them is on another machine. |
| `octok repo set <repo> --alias <alias>` | Sets or changes the alias. `--no-alias` removes it. |
| `octok repo set <repo> --base <branch>` | Changes the base branch. `--no-base` goes back to the base branch of the project. |
| `octok repo set <repo> --url <url>` | Changes the URL and fetches the clone from it. |
| `octok repo set <repo> --location <location>` | Moves the repository to another machine. `--no-location` goes back to the location of the project. |
| `octok repo rm <repo>` | Unregisters a repository and deletes its clone. Refused while a workspace uses it. |

The options of `octok repo set` can be combined in one command.

When a base branch is not on the remote, `octok` asks for confirmation before using it.
Without a terminal, for example in a script, it refuses instead.

### Workspaces: `octok branch`, `octok checkout`, `octok rm`

A workspace is a folder with one checkout per repository, all on the same branch.
It is named after its branch, with `/` replaced by `-`: the branch `feat/login` lives in the workspace `feat-login`.

| Command | What it does |
|---|---|
| `octok branch <branch> [--location <location>]` | Fetches every repository, creates `<branch>` from its up-to-date base branch and checks it out in a new workspace. Prints the path of the workspace. With `--location`, the whole workspace lives on that machine. |
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

### Locations: `--location`

By default everything lives on this machine, under `~/.octotask`.
A **location** moves the clone and the checkouts of a repository to another machine, for example a VM where the backend runs.
`octok` still runs on this machine only: it calls `ssh <host> git ...` for every repository that lives elsewhere.

A location is written in one of three ways:

| Location | Where the folders are |
|---|---|
| `local` | This machine. |
| `<host>` | The folder `.octotask` in the home of the user on `<host>`. `octok` connects once and asks the machine for that home. |
| `<host>:<path>` | The folder `<path>` on `<host>`. A relative path, or one that starts with `~/`, is under the home of the user. |

`<host>` is anything `ssh` accepts: a `Host` of `~/.ssh/config`, or `user@host`.
The user, the port and the key come from your ssh configuration, so `octok` stores none of them.
The project file always holds the absolute path, for example `location: api-dev-1:/home/ubuntu/.octotask`.

The location of a repository is the first one set among these three:

1. the workspace, with `octok branch <branch> --location <location>`: every repository of that workspace lives there;
2. the repository, with `octok repo add --location` or `octok repo set --location`;
3. the project, with `octok project init --location` or `octok project set --location`.

When none is set, the repository lives on this machine.

```console
$ octok repo add git@github.com:acme/shop-backend.git --location api-dev-1
added `shop-backend` to shop: git@github.com:acme/shop-backend.git
cloned at api-dev-1:/home/ubuntu/.octotask

$ octok branch feat/login
created workspace `feat-login` in shop
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend
/Users/you/.octotask/workspaces/shop/feat-login
```

A workspace then has one folder per machine, and `octok status`, `octok pull` and `octok rm` reach all of them.
`octok branch` and `octok checkout` print the folder on this machine, or `<host>:<path>` when the workspace has none here.

**Requirements on the other machine.**
It needs `git` 2.37 or newer, and it must reach the git remotes by itself: give it its own key, or set `ForwardAgent yes` for that host in `~/.ssh/config`.

**Moving a repository.**
A repository changes location only while no workspace uses it, because its checkouts would be left behind.
Its clone is then deleted on the old machine and made again on the new one.
`octok project set --location` is refused while the project has workspaces, for the same reason.

**Leaving a machine.**
`octok project rm` deletes the folders of the project on every machine.
When that was the last project there, the octok folder of that machine goes too, so nothing is left behind.

**A workspace with its own location.**
`octok branch <branch> --location <location>` clones the repositories on that machine when they are not there yet.
The choice is saved in the project file until `octok rm <branch>`, which also deletes the clones made only for that workspace.

**Passphrases: `octok set-ssh-passphrase [<host>]`.**
Type the passphrase of an ssh key once instead of at every command.
The command finds the key files `ssh` uses for `<host>` and gives them to `ssh-add`, which asks for the passphrase and keeps it in the ssh agent: `octok` never reads it and never stores it.
Without `<host>`, it does this for every other machine of the project.
On macOS the passphrase also goes into the keychain, so it survives a reboot.
On Linux it lasts as long as the ssh agent of your session.

**Without a terminal.**
In a script or under an agent nobody can type a passphrase or accept a host key, so `octok` runs `ssh` with `BatchMode=yes`: a command that would have to ask fails at once instead of waiting.

**Connections.**
`octok` shares one ssh connection among the commands of a run and keeps it open for a minute, through a control socket in `~/.ssh/octok-*`.

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

`octok status [--json]` shows the state of the project.
It does not contact the git remotes, and it connects over `ssh` only to the other machines of the project:

- every repository, with its base branch and the last commit of that branch at the last fetch;
- every workspace, with the branch and the last commit of each checkout;
- the problems it finds: a repository that was never fetched, a base branch missing on the remote, a repository with no checkout in a workspace, a checkout that cannot be read, and clones left behind by repositories that are no longer in the project.

## Development

| Command | What it does |
|---|---|
| `make format` | Formats the code. |
| `make test` | Runs the tests. They need `git` and `gh`. A fake `ssh` stands in for the other machine, so they use no network. |
| `make build` | Builds the release binary. |
| `make install` | Installs `octok` into `~/.cargo/bin`. |
| `make clean-data` | Asks for confirmation, then deletes `~/.octotask` and the folders of its projects on the other machines, unpushed work in the workspaces included. It stops before deleting `~/.octotask` when a machine cannot be reached. |
