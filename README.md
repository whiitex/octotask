# Octotask

`octok` runs the same git operation on every repository of a project.

A feature often touches several repositories at once: a frontend, a backend, an agent.
Working on it means creating the same branch in each of them, keeping them up to date and remembering where each one stands.
`octok` groups those repositories into a **project** and gives each feature a **workspace**: one folder with a checkout of every repository, all on the same branch.

- **One command for all repos.** `octok branch feat/login` creates the branch in every repository, starting from a freshly fetched base branch.
- **A teammate's branch in one command.** `octok checkout feat/login` builds the workspace of a branch that already exists: each repository that has the branch gets it, the others start from their base.
- **Isolated workspaces.** Each workspace is a set of `git worktree`s, so several features can be open side by side without stashing or switching branches.
- **No repeated downloads.** Each repository is cloned once into a cache, and every workspace is created from it in an instant.
- **Plain git underneath.** `octok` calls your own `git`, so your credentials, SSH config and hooks apply, and every checkout is a normal git working tree.
- **Repos on other machines.** A repository can live on another machine, such as a VM, reached over `ssh`: `octok` runs the same commands there.
- **One command to start everything.** `octok up` starts every repository of a workspace, shows their output together and stops them all at `Ctrl-C`.
- **Dev changes that follow you.** The local-only edits a repository needs to run on your machine, and files such as `.env`, are saved once, put into every new workspace and kept out of every commit.

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
  shop-frontend (frontend)  staging  4f2a91c  Add cart page (2 days ago)
  shop-backend (backend)    staging  a81d0e3  Fix order totals (5 hours ago)

workspaces:
  feat-login  /Users/you/.octotask/workspaces/shop/feat-login
    shop-frontend (frontend)  feat/login  4f2a91c  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/login  a81d0e3  Fix order totals (5 hours ago)  [not on the remote]
```

## Installation

Requirements:

- macOS or Linux.
- `git` 2.37 or newer.
- `ssh`, only to keep repositories on another machine. See [Locations](#locations---location).

### From a release

Every [release](https://github.com/whiitex/octotask/releases) has one archive per platform:

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `octok-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `octok-x86_64-apple-darwin.tar.gz` |
| Linux, arm64 | `octok-aarch64-unknown-linux-musl.tar.gz` |
| Linux, x86_64 | `octok-x86_64-unknown-linux-musl.tar.gz` |

Download the archive of the latest release, unpack it and move `octok` into a folder of your `PATH`:

```sh
curl -fsSL https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz | tar -xz
sudo mkdir -p /usr/local/bin
sudo mv octok-aarch64-apple-darwin/octok /usr/local/bin/
```

The Linux binaries are statically linked, so they run on any distribution.
The file `SHA256SUMS` of the release has the checksum of every archive.

### From source

Building needs [Rust](https://rustup.rs) 1.89 or newer.
Clone the repository and install the `octok` binary into `~/.cargo/bin`:

```sh
git clone https://github.com/whiitex/octotask.git
cd octotask
make install
```

`make install` runs `cargo install --path crates/octotask-cli --locked`.

### After installing

Check the installation with `octok --version`.

`octok` keeps everything it manages under `~/.octotask`.
Set the `OCTOTASK_HOME` environment variable to use another folder.

| Path | Content |
|---|---|
| `~/.octotask/config.yaml` | The default project. |
| `~/.octotask/projects/<project>.yaml` | The repositories of a project. |
| `~/.octotask/cache/<project>/<repo>.git` | The bare clone of a repository. |
| `~/.octotask/workspaces/<project>/<workspace>/<repo>` | The checkouts of a workspace. |
| `~/.octotask/dev/<project>/<repo>` | The dev changes saved for a repository. |

## Features

Every command acts on the default project.
Pass `-p <project>` to act on another one, for example `octok -p shop status`.

Commands that list data also accept `--json`, for scripts and agents.
Every JSON output carries `"schema": 1`.

### Projects: `octok project`

A project is a named group of repositories with a default base branch.

| Command | What it does |
|---|---|
| `octok project init <name> [--base <branch>] [--location <location>]` | Creates a project and makes it the default. The base branch defaults to `staging`, the location to this machine. |
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
| `octok repo set <repo> --setup <command>` `--run <command>` | Set the commands that `octok up` runs to prepare and start the repository: see [Running](#running-octok-up). |
| `octok repo rm <repo>` | Unregisters a repository and deletes its clone. Refused while a workspace uses it. |

The options of `octok repo set` can be combined in one command.

When a base branch is not on the remote, `octok` asks for confirmation before using it.
Without a terminal, for example in a script, it refuses instead.

### Questions at a terminal

At a terminal, `octok project init` and `octok repo add` ask for each setting that has no flag, and show its default between brackets.
Press Enter to keep the default.
An answer that cannot be used is explained and asked again.

```console
$ octok repo add git@github.com:acme/shop-backend.git
Alias [shop-backend]: backend
Machines in ~/.ssh/config:
  api-dev-1
  bastion
Location [local, project default]: api-dev-1
ssh reaches `api-dev-1` as ubuntu@10.0.0.5, port 22, key /Users/you/.ssh/id_ed25519
Branches on the remote:
  main (default)
  staging
  feat/checkout
  fix/order-totals
Base branch [staging, project default]:
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
cloned at api-dev-1:/home/ubuntu/.octotask
```

- `octok project init` asks for the base branch and the location.
- `octok repo add` asks for the alias, the location and the base branch.
  Enter on the alias saves no alias, and Enter on the location or the base branch makes the repository follow the project.
- The name of a repository is never asked: it is the one in the URL, or `--name`.
- Before the location, `octok` lists the machines `~/.ssh/config` names one by one, the files it includes covered.
  Patterns such as `*.example.com` are left out.
- A location on another machine is tried at once, and `octok` shows the user, the address, the port and the key files `ssh` uses for it.
- The base branch is asked once the repository is cloned, after a list of the branches of its remote:
  the default branch first, then the staging branch under the names teams use for it (`staging`, `stage`, `stg`, `develop`, `development`, `dev`, `next`, `integration`, `preprod`, `pre-prod`), then the 5 other branches with the most recent commits.
- With every flag given, or without a terminal, nothing is asked.

### Workspaces: `octok branch`, `octok checkout`, `octok rm`

A workspace is a folder with one checkout per repository, all on the same branch.
It is named after its branch, with `/` replaced by `-`: the branch `feat/login` lives in the workspace `feat-login`.

| Command | What it does |
|---|---|
| `octok branch <branch> [--location <location>] [--allow-stale-dev]` | Fetches every repository, creates `<branch>` from its up-to-date base branch and checks it out in a new workspace, with the [dev changes](#dev-changes-octok-dev) of each repository. Prints the path of the workspace. With `--location`, the whole workspace lives on that machine. |
| `octok checkout -b <branch>` | Same as `octok branch <branch>`. |
| `octok checkout <branch> [--location <location>] [--allow-stale-dev]` | Prints the path of the workspace of `<branch>`. Creates it first when the branch is already on a remote: see [Branches that already exist](#branches-that-already-exist). |
| `octok rm <branch> [--force]` | Deletes a workspace: its checkouts, its folder and its local branches. |

A program cannot change the folder of the shell that started it, so enter a workspace with `cd`:

```sh
cd "$(octok branch feat/login)"     # create a new branch and enter
cd "$(octok checkout feat/login)"   # enter the workspace of an existing branch
```

**Publishing a branch.**
The new branch exists only on your machine.
A plain `git push` inside a checkout publishes it and sets its upstream, with no need for `-u origin <branch>`.

**Completing a workspace.**
Run `octok checkout <branch>` on an existing workspace to add the checkouts it lacks.
This brings in a repository registered after the workspace was created, finishes a creation that was interrupted, and restores a checkout whose folder was deleted by hand, with its commits.
`octok branch <branch>` does the same, as long as the branch is on no remote.

**Two branches, one folder.**
`feat/login` and `feat-login` give the same workspace name, so only one of them can have a workspace: the second is refused.

**Removing safely.**
`octok rm` refuses to delete a workspace that holds uncommitted changes, unpushed commits or files that do not belong to a checkout, and lists what it found.
`--force` deletes the workspace anyway and discards that work.
Files ignored by git, such as `.env`, are deleted without a warning, and so are the dev changes: they are saved elsewhere.

#### Branches that already exist

`octok checkout <branch>` is for a branch that someone already pushed, such as the pull request of a teammate or of an agent:

```console
$ cd "$(octok checkout feat/login)"
created workspace `feat-login` in shop
  branch from the remote: shop-backend
  new branch from the base: shop-frontend
```

- **A repository that has the branch on its remote** gets a local branch at the same commit, which follows the remote one: `git pull`, `git push` and `octok pull` work on it.
- **A repository that does not have it** gets a new branch with that name from its up-to-date base, as with `octok branch`, so the workspace is complete and `octok up` can start everything.
  The branch stays on your machine until you push it.
  If the branch is pushed to that repository later, `octok pull` makes the local branch follow it.
- **A workspace that is already complete** is left alone: the command only prints its path, without a fetch.
- **A workspace that lacks some checkouts** gets them.
- **A branch that is on no remote** is refused: create it with `octok branch <branch>` or `octok checkout -b <branch>`.

`octok branch` refuses a branch that is already on the remote of a repository, so a new name never lands on someone else's work by mistake.

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

### Running: `octok up`

A repository can say how it is started, with two shell commands:

- the **run command** starts it, such as `npm run dev`;
- the **setup command** prepares a checkout before its first run, such as `npm ci`.

| Command | What it does |
|---|---|
| `octok repo set <repo> --run <command>` | Sets the run command of a repository. `--no-run` removes it. |
| `octok repo set <repo> --setup <command>` | Sets the setup command of a repository. `--no-setup` removes it. |
| `octok repo set <repo> --run <command> --branch <branch>` | Sets the run command for the workspace of one branch only, for example to use another port there. `--no-run --branch <branch>` goes back to the command of the repository. |
| `octok up [<repo>...] [--branch <branch>]` | Starts the repositories of a workspace and stays attached until they have all ended. Without `<repo>`, it starts every repository that has a run command. Without `--branch`, it uses the workspace of the current folder. |

```console
$ octok repo set frontend --setup 'npm ci' --run 'npm run dev -- --port 3000'
$ octok repo set backend --run 'PORT=9005 uv run main.py'

$ octok up --branch feat/login
frontend | $ npm ci
backend  | $ PORT=9005 uv run main.py
backend  | Uvicorn running on http://0.0.0.0:9005
frontend | added 312 packages in 9s
frontend | $ npm run dev -- --port 3000
frontend | Local: http://localhost:3000/
^Cstopping...
backend  | stopped
frontend | stopped
```

- **Where a command runs.** In the checkout of its repository in that workspace, with `sh`, on the machine where the checkout lives. A repository without a run command is not started.
- **Setup.** The setup command runs before the run command, until it succeeds once in that checkout, and again when its text changes. When it fails, the repository is not started.
- **Output.** Every line a repository prints, on stdout or stderr, goes to the stdout of `octok` with the alias or the key of the repository in front. A line that starts with `$` is a command that `octok` runs. What `octok` itself says goes to stderr.
- **A command that ends.** It does not stop the others. `octok up` ends when every repository has ended, with code 0 if they all ended well and 1 otherwise.
- **Stopping.** `Ctrl-C` stops every repository, and `octok up` then exits with code 130. A command and every process it started receive `SIGTERM`, and `SIGKILL` if they are still running 10 seconds later.
- **No process left behind.** The same stop happens by itself when `octok` is killed or its terminal is closed.
- **Repositories on another machine.** Their commands run there, over `ssh`, and stop in the same way. They also stop when they have not heard from `octok` for 30 seconds, which is what a lost connection looks like from the other machine. On its side, `octok` gives up on a machine that has not answered for 15 seconds.

### Dev changes: `octok dev`

Some changes are needed to run a repository on your machine and must never be committed: a port, a database name, a line turned off, a `.env` file.
`octok` calls them **dev changes**.
Save them once, and every new workspace gets them, while git never sees them.

| Command | What it does |
|---|---|
| `octok dev save [<repo>...] [--file <path>]... [--branch <branch>]` | Saves the dev changes of a workspace from its checkouts, replacing the ones saved before. Without `<repo>`, it saves every repository. Without `--branch`, it uses the workspace of the current folder. |
| `octok dev show [<repo>...] [--json]` | Shows what is saved for each repository: the files, and the changes to the tracked ones. |
| `octok branch <branch> --allow-stale-dev` | Creates a workspace even when some dev changes no longer apply. See below. |

```console
$ cd "$(octok checkout feat/login)/shop-backend"
$ $EDITOR main.py                      # port 8000 becomes 9005
$ echo 'TOKEN=...' > .env

$ octok dev save --file .env
shop-backend (backend)
  changed  main.py
  new      .env

$ git status --short                   # nothing: git no longer sees them

$ cd "$(octok branch feat/cart)/shop-backend"
$ grep port main.py                    # the new workspace has them
    run(port=9005)
```

- **What is saved.** For each repository: every change to a tracked file that is not committed, and every file that git does not track.
  A file that git ignores, such as `.env`, is saved only when it is named with `--file`, and then stays saved for as long as it is in the checkout.
  Commit your real work before saving: whatever is not committed counts as a dev change.
- **Changing them.** Edit the files in a checkout and run `octok dev save` again: what is saved follows the checkout.
  Saving from a checkout with no dev changes left removes them.
- **New workspaces.** `octok branch` and `octok checkout` put the dev changes in every checkout they create.
  The workspaces that already exist keep what they have.
- **Commits.** A tracked file with dev changes can still be edited and committed: the commit holds your edit and not the dev changes, which stay in the file on disk.
  `git status` and `git diff` do not show them: `octok dev show` does.
  The files that git does not track are ignored by git in every checkout.
- **Lines that clash.** A change to the very lines of a dev change cannot be told apart from it, so git refuses to stage that file and says so.
  If it is a new dev change, run `octok dev save`.
- **A branch that changed the same lines.** The dev changes of that file can no longer be applied to the commit a checkout starts from, and `octok branch` or `octok checkout` creates nothing and names the file.
  `--allow-stale-dev` creates the workspace with that file as it is in git: make the dev changes again there, then run `octok dev save`.
- **Where they are kept.** Under the `octok` home of the machine where the repository lives, in `dev/<project>/<repo>`, so a `.env` never leaves that machine.
  Inside, `dev/changed` holds the tracked files with their dev changes, `dev/new` the files that git does not track, and `base` the tracked files as git had them when they were saved.
  They are deleted with the repository and with the project, and when the repository moves to another machine.
- **How it works.** `octok` sets up a git filter in the clone of the repository, for the files with dev changes only.
  git runs it when it writes one of those files, to put the dev changes in, and when it reads one, to take them out.

### Overview: `octok status`

`octok status [--json]` shows the state of the project.
It does not contact the git remotes, and it connects over `ssh` only to the other machines of the project:

- every repository, with its base branch and the last commit of that branch at the last fetch;
- every workspace, with the branch and the last commit of each checkout, marked `[not on the remote]` when the branch was not on the remote at the last fetch;
- the problems it finds: a repository that was never fetched, a base branch missing on the remote, a repository with no checkout in a workspace, a checkout that cannot be read, and clones left behind by repositories that are no longer in the project.

## Development

| Command | What it does |
|---|---|
| `make format` | Formats the code. |
| `make test` | Runs the tests. They need `git` and `gh`. A fake `ssh` stands in for the other machine, so they use no network. |
| `make build` | Builds the release binary. |
| `make install` | Installs `octok` into `~/.cargo/bin`. |
| `make clean-data` | Asks for confirmation, then deletes `~/.octotask` and the folders of its projects on the other machines, unpushed work in the workspaces included. It stops before deleting `~/.octotask` when a machine cannot be reached. |

### Releases

Every merge into `main` is released: the `Release` workflow tags the merge commit `v<version>`, builds `octok` for the four platforms and publishes a GitHub release with the archives.
The version is the `version` of the root `Cargo.toml`, shared by both crates.
A pull request to `main` fails its checks unless that version is a plain `X.Y.Z` higher than every released one, so raise it before opening the pull request.
