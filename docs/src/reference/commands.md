# Command reference

Every command of `octok` with its arguments and options, as its `--help` describes them.

The options of every command are listed once, under [`octok`](#octok).

## `octok`

octotask: multi-repo workspaces: one command for branches, PRs and services across repos

```text
Usage: octok [OPTIONS] <COMMAND>
```

Options of every command:

- `-p, --project <PROJECT>`: Project to act on (defaults to the one set with `octok project use`)
- `--json`: Print one JSON document on stdout instead of text, and never ask a question
- `-h, --help`: Print help

Options:

- `-V, --version`: Print version

Commands:

- [`octok project`](#octok-project): Manage projects (groups of repos)
- [`octok repo`](#octok-repo): Manage the repos registered in a project
- [`octok fetch`](#octok-fetch): Fetch every repo of the project, cloning the ones not cached yet
- [`octok branch`](#octok-branch): Create a workspace: one worktree per repo, on a new branch from the up-to-date base
- [`octok rm`](#octok-rm): Delete a workspace: its worktrees, its folder and the local branches
- [`octok checkout`](#octok-checkout): Print the path of the workspace of a branch, creating it when the branch is on a remote
- [`octok pull`](#octok-pull): Fetch, then fast-forward each worktree of a workspace to its remote branch
- [`octok up`](#octok-up): Start the repos of a workspace, each with its `run` command, until Ctrl-C
- [`octok dev`](#octok-dev): Manage the dev changes: what every new workspace gets and no commit contains
- [`octok pr`](#octok-pr): Work with pull requests
- [`octok status`](#octok-status): Show the project, its repos and the checkouts of every workspace
- [`octok set-ssh-passphrase`](#octok-set-ssh-passphrase): Type the passphrase of an ssh key once: `ssh-add` asks for it and keeps it
- [`octok docs`](#octok-docs): Print the documentation of this version of octok: the list of its pages, or one page

## `octok project`

Manage projects (groups of repos)

```text
Usage: octok project [OPTIONS] <COMMAND>
```

Commands:

- [`octok project init`](#octok-project-init): Create a new project and make it the default
- [`octok project ls`](#octok-project-ls): List projects
- [`octok project use`](#octok-project-use): Set the default project
- [`octok project set`](#octok-project-set): Change the settings of the project
- [`octok project rm`](#octok-project-rm): Delete a project, its list of repos and their clones

## `octok project init`

Create a new project and make it the default

```text
Usage: octok project init [OPTIONS] <NAME>
```

Arguments:

- `<NAME>`: Name of the new project

Options:

- `--base <BASE>`: Default base branch for the project's repos [default: staging]
- `--location <LOCATION>`: Machine where the project's repos live: `local` (the default), `<host>` or `<host>:<path>`

## `octok project ls`

List projects

```text
Usage: octok project ls [OPTIONS]
```

## `octok project use`

Set the default project

```text
Usage: octok project use [OPTIONS] <NAME>
```

Arguments:

- `<NAME>`: Project to make the default

## `octok project set`

Change the settings of the project

```text
Usage: octok project set [OPTIONS] <--base <BASE>|--location <LOCATION>|--no-location>
```

Options:

- `--base <BASE>`: Default base branch for the project's repos
- `--location <LOCATION>`: Machine where the project's repos live: `local`, `<host>` or `<host>:<path>`
- `--no-location`: Go back to this machine

## `octok project rm`

Delete a project, its list of repos and their clones

```text
Usage: octok project rm [OPTIONS] <NAME>
```

Arguments:

- `<NAME>`: Project to delete

Options:

- `--force`: Also delete its workspaces, with their uncommitted changes and unpushed commits

## `octok repo`

Manage the repos registered in a project

```text
Usage: octok repo [OPTIONS] <COMMAND>
```

Commands:

- [`octok repo add`](#octok-repo-add): Register a git repo in the project and clone it
- [`octok repo ls`](#octok-repo-ls): List the project's repos
- [`octok repo set`](#octok-repo-set): Change the alias, the base branch, the url, the location or the commands of a repo
- [`octok repo rm`](#octok-repo-rm): Unregister a repo, given by key or alias, and delete its clone

## `octok repo add`

Register a git repo in the project and clone it

```text
Usage: octok repo add [OPTIONS] <URL>
```

Arguments:

- `<URL>`: Git url of the repo, as `git clone` takes it

Options:

- `--name <NAME>`: Key for the repo, also its folder name (defaults to the url's last path segment)
- `--alias <ALIAS>`: Short name accepted wherever the key is, e.g. `frontend`
- `--base <BASE>`: Base branch for this repo, overriding the project's
- `--location <LOCATION>`: Machine where this repo lives, overriding the project's: `local`, `<host>` or `<host>:<path>`

## `octok repo ls`

List the project's repos

```text
Usage: octok repo ls [OPTIONS]
```

## `octok repo set`

Change the alias, the base branch, the url, the location or the commands of a repo

```text
Usage: octok repo set [OPTIONS] <--alias <ALIAS>|--no-alias|--base <BASE>|--no-base|--url <URL>|--location <LOCATION>|--no-location|--setup <SETUP>|--no-setup|--run <RUN>|--no-run|--branch <BRANCH>> <REPO>
```

Arguments:

- `<REPO>`: Key or alias of the repo

Options:

- `--alias <ALIAS>`: Short name accepted wherever the key is, e.g. `frontend`
- `--no-alias`: Remove the alias
- `--base <BASE>`: Base branch for this repo, overriding the project's
- `--no-base`: Go back to the project's base branch
- `--url <URL>`: New git url: the clone is fetched from it
- `--location <LOCATION>`: Machine where this repo lives, overriding the project's: `local`, `<host>` or `<host>:<path>`
- `--no-location`: Go back to the project's location
- `--setup <SETUP>`: Shell command that `octok up` runs in a checkout before its first run, e.g. `npm ci`
- `--no-setup`: Remove the setup command
- `--run <RUN>`: Shell command that `octok up` runs in the checkout to start the repo
- `--no-run`: Remove the run command
- `--branch <BRANCH>`: Change the run command in the workspace of this branch only

## `octok repo rm`

Unregister a repo, given by key or alias, and delete its clone

```text
Usage: octok repo rm [OPTIONS] <KEY>
```

Arguments:

- `<KEY>`: Key or alias of the repo

## `octok fetch`

Fetch every repo of the project, cloning the ones not cached yet

```text
Usage: octok fetch [OPTIONS]
```

## `octok branch`

Create a workspace: one worktree per repo, on a new branch from the up-to-date base

On a workspace that already exists, adds the worktrees it lacks: this completes an interrupted creation and brings in the repos registered later.
Refuses a branch that is already on a remote: `octok checkout <branch>` is for that.

```text
Usage: octok branch [OPTIONS] <NAME>
```

Arguments:

- `<NAME>`: Name of the new branch

Options:

- `--location <LOCATION>`: Machine where the whole workspace lives, whatever the repos and the project say: `local`, `<host>` or `<host>:<path>`
- `--allow-stale-dev`: Create the workspace even if some dev changes no longer apply: the files they are for are left as they are in git

## `octok rm`

Delete a workspace: its worktrees, its folder and the local branches

```text
Usage: octok rm [OPTIONS] <BRANCH>
```

Arguments:

- `<BRANCH>`: Branch of the workspace

Options:

- `--force`: Also discard uncommitted changes and unpushed commits

## `octok checkout`

Print the path of the workspace of a branch, creating it when the branch is on a remote

Use it as `cd "$(octok checkout feat/login)"`.
When the workspace is not there yet, a repo that has the branch on its remote gets it, and any other repo gets a new branch from its base.
On a workspace that lacks some worktrees, adds them.

```text
Usage: octok checkout [OPTIONS] <BRANCH>
```

Arguments:

- `<BRANCH>`: Branch of the workspace

Options:

- `-b`: Create a new branch, like `octok branch`
- `--location <LOCATION>`: Machine where the whole workspace lives, whatever the repos and the project say: `local`, `<host>` or `<host>:<path>`
- `--allow-stale-dev`: Create the workspace even if some dev changes no longer apply: the files they are for are left as they are in git

## `octok pull`

Fetch, then fast-forward each worktree of a workspace to its remote branch

```text
Usage: octok pull [OPTIONS] [BRANCH]
```

Arguments:

- `[BRANCH]`: Branch of the workspace (defaults to the workspace of the current directory)

## `octok up`

Start the repos of a workspace, each with its `run` command, until Ctrl-C

Every line a repo prints is shown with the name of the repo in front.
A repo without a `run` command is not started.

```text
Usage: octok up [OPTIONS] [REPOS]...
```

Arguments:

- `[REPOS]...`: Keys or aliases of the repos to start (defaults to every repo with a `run` command)

Options:

- `-b, --branch <BRANCH>`: Branch of the workspace (defaults to the workspace of the current directory)

## `octok dev`

Manage the dev changes: what every new workspace gets and no commit contains

```text
Usage: octok dev [OPTIONS] <COMMAND>
```

Commands:

- [`octok dev save`](#octok-dev-save): Save the dev changes of a workspace: what its checkouts have and git does not
- [`octok dev show`](#octok-dev-show): Show the dev changes saved for the repos

## `octok dev save`

Save the dev changes of a workspace: what its checkouts have and git does not

For each repo: every change that is not committed, and every file git does not track.
What was saved before for the repo is replaced.

```text
Usage: octok dev save [OPTIONS] [REPOS]...
```

Arguments:

- `[REPOS]...`: Keys or aliases of the repos (defaults to every repo of the workspace)

Options:

- `--file <PATH>`: A file that git ignores, to save all the same, e.g. `.env`: can be repeated
- `-b, --branch <BRANCH>`: Branch of the workspace (defaults to the workspace of the current directory)

## `octok dev show`

Show the dev changes saved for the repos

```text
Usage: octok dev show [OPTIONS] [REPOS]...
```

Arguments:

- `[REPOS]...`: Keys or aliases of the repos (defaults to every repo of the project)

## `octok pr`

Work with pull requests

```text
Usage: octok pr [OPTIONS] <COMMAND>
```

Commands:

- [`octok pr review`](#octok-pr-review): Create a workspace to try pull requests, with each repo on the branch you choose

## `octok pr review`

Create a workspace to try pull requests, with each repo on the branch you choose

A repo given with `--pr` gets the branch of that pull request, one given with `--branch` gets that branch, and any other repo gets a new branch from its base.
Without `--pr` and `--branch`, asks at a terminal what to take for each repo.
Prints the path of the workspace.

Pull requests are read with `gh`, which must be signed in.
One that comes from a fork is refused.
What the workspace reviews is remembered until `octok rm`.

```text
Usage: octok pr review [OPTIONS]
```

Options:

- `--pr <REPO=NUMBER>`: A pull request to review, as the key or alias of its repo and its number, e.g. `backend=12`: can be repeated
- `--branch <REPO=BRANCH>`: A branch to review, as the key or alias of its repo and its name, e.g. `agent=fix/prompt`: can be repeated
- `--name <NAME>`: Name of the workspace, and of the branch of the repos left on their base (defaults to `pr-<repo>-<number>` of the first pull request, else to the first branch, in the order of the repos of the project)
- `--location <LOCATION>`: Machine where the whole workspace lives, whatever the repos and the project say: `local`, `<host>` or `<host>:<path>`
- `--allow-stale-dev`: Create the workspace even if some dev changes no longer apply: the files they are for are left as they are in git

## `octok status`

Show the project, its repos and the checkouts of every workspace

```text
Usage: octok status [OPTIONS]
```

## `octok set-ssh-passphrase`

Type the passphrase of an ssh key once: `ssh-add` asks for it and keeps it

```text
Usage: octok set-ssh-passphrase [OPTIONS] [HOST]
```

Arguments:

- `[HOST]`: Machine whose key to unlock (defaults to every other machine of the project)

## `octok docs`

Print the documentation of this version of octok: the list of its pages, or one page

Without a topic, lists every page with its topic, title and summary.
With a topic, prints that page as Markdown.
Needs no project.

```text
Usage: octok docs [OPTIONS] [TOPIC]
```

Arguments:

- `[TOPIC]`: Page to print, as `octok docs` lists it, e.g. `commands/project`
