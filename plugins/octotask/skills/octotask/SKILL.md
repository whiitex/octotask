---
name: octotask
description: Drives octok, the octotask command that gives a feature one workspace with a checkout of every git repository of a project, all on the same branch. Use it when a task touches more than one repository of a project, for example to start a feature across a frontend and a backend, to open a teammate's branch or a pull request in every repository, to pull, start or remove a multi-repository workspace, or to find where each repository of a project is checked out.
license: MIT
compatibility: Needs a shell, git 2.37 or newer and octok, on macOS or Linux.
---

# octotask

`octok` groups git repositories into a **project** and gives each feature a **workspace**: one folder with a **checkout** of every repository, all on the same branch.
A checkout is a normal git working tree, so you work in it with plain `git`.

Use `octok` when the task touches more than one repository of a project.
For a single repository that is not in a project, use `git` alone.

## Before you start

1. Check that `octok` is installed with `octok --version`.
   If it is not, ask the user before installing it, then follow [Install](#install).
2. Read what is there with `octok status --json`.
   - Exit code 2 with `error.code` `no_default_project` means no project is chosen: list them with `octok project ls --json` and pass one with `-p <project>`, as in `octok -p shop status --json`.
   - A project that is not listed is not registered: ask the user.
     Do not create projects or register repositories on your own.

## Rules

- **Always pass `--json`.**
  The command then prints one JSON document on stdout, asks no question and never waits for an answer.
  The text without `--json` is for people and changes between releases.
- **Read stdout, not stderr.**
  stderr carries the progress of git.
- **Check the exit code first.**
  0 is success.
  Otherwise stdout holds `{"schema":1,"error":{"code":...,"message":...}}`: decide on `error.code`, and show `error.message` to the user when you cannot go on.
- **A workspace is named after its branch**, with every `/` turned into `-`: the branch `feat/login` lives in the workspace `feat-login`.
  Commands take the branch, never the name of the workspace.
  `octok branch`, `octok checkout`, `octok pull` and `octok rm` take it as their argument, `octok up`, `octok open` and `octok dev save` as `--branch <branch>`.
- **Look with `octok status --json`.**
  It only reads, while `octok checkout` creates the workspace it is asked about.

| Exit | Meaning | What to do |
|---|---|---|
| 0 | Done. | |
| 1 | Another error, such as a git command that failed. | Read `error.message`. |
| 2 | The command is not written the way `octok` takes it. | Fix the command. |
| 3 | A project, repository, branch, workspace, run command or documentation page it needs is not there. | Check the name with `octok status --json`. |
| 4 | Refused: it would lose work that exists nowhere else. | Commit and push, or ask the user. |
| 5 | Refused: it conflicts with what exists. | Read `error.code`. |
| 6 | At least one repository failed. The document is the normal one. | Read `error` in each entry of `repos`. |
| 7 | Another machine or a program cannot be reached. | Ask the user. |

The document of every command and every `error.code` are in the page `agents/json-output`: read it with `octok docs agents/json-output`.

`octok docs` lists the pages of the documentation of the installed `octok`, and `octok docs <topic>` prints one as Markdown.
The pages describe the version that prints them, so prefer them to what you remember of `octok`.
Reading a page is the one exception to `--json`: the page is the same Markdown either way, and easier to read without it.

## The usual flow

1. **See the project.**

   ```sh
   octok status --json
   ```

   `repos` are the repositories with their `key`, `alias` and base branch.
   `workspaces` are the workspaces that exist, each with the `path` and the `branch` of its `checkouts`.

2. **Open the workspace of the branch.**

   For a new branch:

   ```sh
   octok branch feat/login --json
   ```

   It fetches every repository and creates the branch from the up-to-date base branch of each one.
   Exit 5 with `branch_exists` means a remote already has that branch, and `workspace_exists` means the workspace is already there: in both cases use `octok checkout` instead.
   `octok checkout -b feat/login --json` is the same command under another name, and some error messages suggest it.

   For a branch that exists, such as the pull request of a teammate, or to go back to a workspace:

   ```sh
   octok checkout feat/login --json
   ```

   A repository whose remote has the branch gets it, and any other repository gets a new branch with that name from its base.
   On a workspace that is already there it changes nothing and prints its paths again.
   It creates the workspace whenever a remote has the branch, even one you removed a moment ago, so do not use it only to look.
   Exit 3 with `branch_not_found` means no remote has the branch: create it with `octok branch`.

   Both print where each repository is checked out, and what was done for it:

   ```json
   {"schema":1,"project":"shop","workspace":"feat-login","path":"/Users/you/.octotask/workspaces/shop/feat-login","repos":[{"key":"shop-frontend","path":"/Users/you/.octotask/workspaces/shop/feat-login/shop-frontend","outcome":"created"},{"key":"shop-backend","path":"/Users/you/.octotask/workspaces/shop/feat-login/shop-backend","outcome":"tracked"}]}
   ```

   | `outcome` | Meaning |
   |---|---|
   | `created` | A new branch from the base branch: the remote did not have the branch. |
   | `tracked` | The branch that the remote already had. |
   | `restored` | The branch was still in the clone, without a checkout. |
   | `kept` | The checkout was already there. A second run says `kept` for every repository. |

3. **Work in each checkout**, at `repos[].path`, with the usual tools: edit, test, `git add`, `git commit`.
   Touch only the repositories the task needs.
   The others stay on their branch with no commit, and that is fine.

4. **Publish** with a plain `git push` inside each checkout you changed.
   It sets the upstream by itself.
   `octok` never pushes and never opens a pull request: use `git` and `gh` for that, once per repository.

5. **Update** a workspace that others push to:

   ```sh
   octok pull feat/login --json
   ```

   It fetches and fast-forwards each checkout, and never merges, rebases or resets.
   Inside a checkout, `octok pull --json` alone updates the workspace of that folder.
   Read `repos[].outcome`: `updated` and `up_to_date` are fine, `no_remote_branch` means the branch was never pushed for that repository, or was deleted on the remote, and `diverged` means that checkout needs a merge or a rebase, which is yours to do with `git`.

6. **Remove** the workspace when the work is merged or pushed:

   ```sh
   octok rm feat/login --json
   ```

   Exit 4 with `workspace_has_work` means it holds uncommitted changes, unpushed commits, or other files next to the checkouts.
   Each line of `error.reasons` names a repository and what it holds, not the files: run `git status` in that checkout to see them.
   The line `other files in the workspace: ...` names the files that sit in the workspace folder and belong to no checkout.
   Commit and push what belongs to the task, and ask the user about anything else.
   Leave a workspace you did not create alone unless the user asks.

## Reading a path

- A `path` that starts with `/` is a folder on this machine: `cd` into it.
- A `path` written `<host>:<path>`, such as `api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend`, is a folder on another machine, reached with `ssh <host>`.
  Run a command there as `ssh api-dev-1 'cd /home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend && git status'`.
  `octok` itself always runs on this machine and reaches the other one by itself.
- The `path` of a workspace is its folder on this machine when it has one here.
  Use the `path` of each entry of `repos` to reach a checkout, never a path you build yourself.

## Never without asking the user

These lose work or change what the user set up, so ask first and run them only on a clear yes:

- `--force`, on `octok rm` and on `octok project rm`: it discards uncommitted changes and unpushed commits.
  An error message may suggest it. That is a hint for the user, not for you.
- `octok project rm <project>`: it deletes the project and the clones of its repositories.
- `octok repo rm <repo>`: it unregisters a repository and deletes its clone.
- Changing a location, with `--location <location>` or `--no-location` on any command: it decides on which machine the code lives.
- `octok project init`, `octok project use`, `octok project set`, `octok repo add` and `octok repo set`: the user decides what a project contains and how it is set up.
  Pass `-p <project>` instead of changing the default project.
- `octok dev save`: it replaces the dev changes saved for a repository, which every new workspace then gets.
- `--allow-stale-dev`: it creates a workspace without the dev changes that no longer apply.
- `octok set-ssh-passphrase`: it needs a person at a terminal. Ask the user to run it when exit 7 comes with `error.code` `ssh`.

## Other commands

| Command | What it does |
|---|---|
| `octok pr review --pr <repo>=<number> --json` | Opens a workspace with a repository on the branch of that pull request and the others on their base. `--pr` and `--branch <repo>=<branch>` can be repeated, and `--name <name>` names the workspace. Needs `gh`, signed in. |
| `octok fetch --json` | Fetches every repository of the project into its clone. |
| `octok repo ls --json` | Lists the repositories with their key, alias, URL, base branch and location. |
| `octok project ls --json` | Lists the projects and the default one. |
| `octok dev show --json` | Shows the dev changes saved for each repository: the local-only edits that every new workspace gets and no commit contains. |
| `octok up --branch <branch> --json` | Starts every repository of the workspace that has a run command, and stays attached until they end. |
| `octok open <editor> --branch <branch> --json` | Opens each repository of the workspace in a window of `code`, `cursor`, `windsurf` or `codium`, a remote one through Remote-SSH. Exit 6 when a window failed. |
| `octok docs <topic>` | Prints a page of the documentation of the installed `octok`. `octok docs` lists the pages. Needs no project. |

A repository is named by its `key` or its `alias` wherever a command takes one.

`octok up` does not return while the repositories run, so start it in the background and read its lines as they come.
Each line is one JSON document.
The first one says whether anything started: it has an `event` `started`, or an `error` when nothing could start.
Then comes `output` for every line a repository prints, and `exited` when one ends.
Stop it with SIGINT. It then stops every repository and exits with 130.
Inside a checkout, `octok up --json` alone starts the workspace of that folder.

The run command of a repository is set by the user and is never printed, so the only way to know that one is missing is to try.
Exit 3 with `nothing_to_run` or `no_run_command` means a repository has none: ask the user for it, and do not make one up.
Exit 3 with `undefined_variable` means an env value reads one that is not set: tell the user which one, from `error.variable`, and do not invent a value.

## Install

Ask the user first.
Every release has one archive per platform:

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `octok-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `octok-x86_64-apple-darwin.tar.gz` |
| Linux, arm64 | `octok-aarch64-unknown-linux-musl.tar.gz` |
| Linux, x86_64 | `octok-x86_64-unknown-linux-musl.tar.gz` |

Download the one for the machine, unpack it and move the binary into a folder of `PATH`:

```sh
curl -fsSL https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz | tar -xz
mkdir -p ~/.local/bin
mv octok-aarch64-apple-darwin/octok ~/.local/bin/
```

Then check it with `octok --version`.
