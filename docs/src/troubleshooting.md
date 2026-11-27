# Troubleshooting

Every error that `octok` reports, by area, with the message, its cause and the fix, then the behaviors that look like errors and are not.

## Reading an error

A failed command prints `error: <message>` on stderr and exits with a code other than 0, the same with and without `--json`.
`octok status || echo "exit $?"` shows the code.
With `--json` the failure is a document on stdout with a stable `code`, listed in [JSON output](agents/json-output.md).
Each error below gives its `code` and exit, so a script can decide on them and not on the text.

| Exit | Meaning |
|---|---|
| 0 | Done. |
| 1 | Another error: git or `gh` failed, a file cannot be read. |
| 2 | The command line is not one `octok` takes: an unknown option, an invalid name, no project. |
| 3 | Something named is not there: a project, repository, branch, workspace, run command or documentation page. |
| 4 | Refused, because it would lose work that exists nowhere else. |
| 5 | Refused, because it conflicts with what exists: already there, in use, or no longer applies. |
| 6 | The command went through the repositories one by one, and at least one failed. |
| 7 | Another machine or a program cannot be reached. |
| 130 | `octok up` was stopped with `Ctrl-C` or a signal. |

When `cd "$(octok branch feat/cart)"` fails, `cd` receives nothing and you stay where you were: read the message before you go on.

The examples use the project `shop`, and each block continues from the one before it.

## Starting out: projects and names

In a clean home only `octok project init`, `octok project ls`, `octok docs` and `--help` work.

| Message | Code, exit | Fix |
|---|---|---|
| `no project given and no default project set ...` | `no_default_project`, 2 | Create a project with `octok project init <name>`, choose one with `octok project use <name>`, or pass `-p <name>` before or after the command. |
| ``project `nope` not found`` | `project_not_found`, 3 | `octok project ls` lists the projects. |
| ``topic `nope` not found ...`` | `topic_not_found`, 3 | `octok docs` lists the topics. |
| `unrecognized subcommand`, `unexpected argument`, `the following required arguments were not provided` | `usage`, 2 | Run the command with `--help`. With `--json` this is a document too. |
| ``invalid <kind> `<value>`: <reason>`` | `invalid_name`, 2 | Follow the reason and the name rules below. |
| ``invalid location `<value>`: <reason>`` | `invalid_location`, 2 | A host has only letters, digits, `.`, `_`, `-` and `@`, and does not start with `-`. In a project file, a location is `local` or `<host>:<absolute path>`. |
| ``project `shop` already exists`` | `project_exists`, 5 | Choose another name, or `octok project use shop`. |
| ``project `shop` has no repos ...`` | `no_repos`, 3 | Add a repository with `octok repo add <url>`. |

`<kind>` is `project name`, `repo key`, `alias`, `branch`, `workspace name`, `file`, or a command such as `run command`.

- **Project names and keys:** at most 64 characters, starting with a letter or a digit, with only letters, digits, `.`, `_` and `-`.
- **Branches:** a name git accepts, not starting with `-`, and so is the `--name` of `octok pr review`.
- **Workspace names:** have no `/`, an error only in a project file edited by hand.
- **Commands:** cannot be empty.

```console
$ octok status
error: no project given and no default project set (use `octok project use <name>`)
$ octok -p nope status
error: project `nope` not found
$ octok docs nope
error: topic `nope` not found: list the topics with `octok docs`
$ octok frobnicate
error: unrecognized subcommand 'frobnicate'

Usage: octok [OPTIONS] <COMMAND>

For more information, try '--help'.
```

```console
$ octok project init 'bad name'
error: invalid project name `bad name`: may only contain letters, digits, `.`, `_` and `-`
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok project init shop
error: project `shop` already exists
$ octok project set --location 'a b:/x'
error: invalid location `a b`: the host may only contain letters, digits, `.`, `_`, `-` and `@`
$ octok branch feat/x
error: project `shop` has no repos: add one with `octok repo add <url>`
```

## Registering repositories

| Message | Code, exit | Fix |
|---|---|---|
| ``invalid git url `<url>` `` | `invalid_url`, 2 | Write `git@github.com:acme/shop-backend.git`, an `https://`, `http://`, `ssh://`, `git://` or `file://` URL, or an absolute path, with no spaces. |
| ``cannot derive a repo key from `<url>` ...`` | `no_repo_key`, 2 | Choose the key with `--name <key>`. |
| ``repo `shop-backend` is already registered ...`` | `duplicate_repo`, 5 | Two repositories have the same name, or one is registered twice: use `--name` for the second. |
| ``alias `backend` is already used by repo `shop-backend` `` | `duplicate_alias`, 5 | Choose another alias: it works wherever a key does. |
| ``repo `nope` is not registered`` | `repo_not_found`, 3 | `octok repo ls` lists the keys and aliases. |
| ``repo `shop-backend` has no branch `nope` on its remote`` | `base_not_found`, 3 | At a terminal `octok` asks whether to use it anyway; without one, or with `--json`, it refuses. Check the name with `git ls-remote --heads <url>`. |

``warning: branch `nope` is not on the remote of: ...`` is not an error: `octok project set --base` saved the base.
`octok status` shows ``branch `nope` is not on the remote`` until the branch exists or you change the base back.
See [Repositories](commands/repo.md).

```console
$ octok repo add git@github.com:acme/shop-backend.git --base nope
error: repo `shop-backend` has no branch `nope` on its remote
$ octok repo add 'not a url'
error: invalid git url `not a url`
$ octok repo add 'git@github.com:acme/.git'
error: cannot derive a repo key from `git@github.com:acme/.git`: choose one with `--name <key>`
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
error: repo `shop-backend` is already registered (git@github.com:acme/shop-backend.git)
choose another name with `--name <key>`
$ octok repo add git@github.com:acme/shop-backend.git --name other --alias backend
error: alias `backend` is already used by repo `shop-backend`
$ octok repo set nope --alias x
error: repo `nope` is not registered
$ octok project set --base nope
warning: branch `nope` is not on the remote of: shop-backend
base of `shop` is now `nope`
$ octok project set --base staging
base of `shop` is now `staging`
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
```

## Creating and finding workspaces

| Message | Code, exit | Fix |
|---|---|---|
| ``no repo has a branch `nope` on its remote ...`` | `branch_not_found`, 3 | `octok checkout` is for a pushed branch: start a new one with `octok branch <branch>`. |
| ``branch `feat/login` already exists on the remote of ...`` | `branch_exists`, 5 | Use `octok checkout`, or another name. |
| ``workspace `feat-cart` already exists at <path>`` | `workspace_exists`, 5 | `cd "$(octok checkout feat/cart)"` goes into it and adds the checkouts it lacks. |
| ``workspace `feat-cart` is the one of the branch `feat/cart` ...`` | `workspace_of_another_branch`, 5 | `feat/cart` and `feat-cart` give the same folder: remove the other workspace, or use another branch name. |
| ``no workspace for `feat-nope` ...`` | `workspace_not_found`, 3 | `octok status` lists the workspaces. A command takes the branch of a workspace, never a folder name. `octok rm` names the branch you gave, the others the workspace. |
| `not inside a workspace ...` | `not_in_workspace`, 2 | Pass the branch, or `cd` into the workspace. |
| ``branch `fix/prompt` of `shop-agent` is already checked out in <path>`` | `branch_in_use`, 5 | A branch lives in one checkout at a time: use that workspace, or remove it first. |

See [Workspaces](commands/workspaces.md).

```console
$ octok checkout nope
error: no repo has a branch `nope` on its remote: create it with `octok checkout -b nope`
$ octok branch feat/login
error: branch `feat/login` already exists on the remote of: shop-backend, shop-frontend
check it out with `octok checkout feat/login`, or choose another name
$ cd "$(octok checkout feat/login)"
created workspace `feat-login` in shop
  branch from the remote: shop-backend, shop-frontend
$ octok branch feat/login
error: workspace `feat-login` already exists at /Users/you/.octotask/workspaces/shop/feat-login
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ octok branch feat/cart
error: workspace `feat-cart` already exists at /Users/you/.octotask/workspaces/shop/feat-cart
$ octok checkout feat-cart
error: workspace `feat-cart` is the one of the branch `feat/cart`, whose name gives the same folder
remove it with `octok rm feat/cart`, or use another branch name
$ cd
$ octok pull
error: not inside a workspace: pass its branch, as in `octok pull <branch>`
$ octok up --branch feat/nope
error: no workspace for `feat-nope`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch
```

## Running

| Message | Code, exit | Fix |
|---|---|---|
| ``no repo of workspace `feat-login` has a `run` command ...`` | `nothing_to_run`, 3 | Set one with `octok repo set <repo> --run <command>`. |
| ``repo `shop-backend` has no `run` command ...`` | `no_run_command`, 3 | Set one for the repository you named. |
| ``env `API` of repo `shop-frontend` uses `{{ repos.backend.env.PORT }}`, which is not defined`` | `undefined_variable`, 3 | Set it with `octok repo set backend --env PORT=<value>`, or with `--branch` for this workspace only. |
| ``env values refer to each other in a loop: shop-backend.A -> shop-backend.B -> ...`` | `env_cycle`, 2 | Change one value of the chain so that it no longer reads the next. |
| ``invalid env `<name>` of repo `<repo>`: <reason>`` | `invalid_env`, 2 | Follow the [rules](commands/up.md#rules) of names, values and templates. |
| ``repo `shop-agent` has no worktree in workspace ...`` | `no_worktree`, 3 | See the next section. |

`<repo> | ended (exit status: N)` says how each repository ended, and a repository whose setup failed is not started: read the output above its `$ <setup command>` line.
See [Running](commands/up.md) for the exit codes.

```console
$ octok up --branch feat/login
error: no repo of workspace `feat-login` has a `run` command: set one with `octok repo set <repo> --run <command>`
$ octok up backend --branch feat/login
error: repo `shop-backend` has no `run` command: set one with `octok repo set shop-backend --run <command>`
```

## A repository added later

A repository registered after a workspace has no checkout in it: `octok up` refuses with `no_worktree`, and `octok status` shows ``no worktree: add it with `octok checkout <branch>` ``.
Run `octok checkout <branch>`, which adds the checkouts the workspace lacks.

```console
$ octok repo add git@github.com:acme/shop-agent.git --alias agent
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
$ octok repo set agent --run true
updated `shop-agent (agent)` in shop
$ octok up agent --branch feat/login
error: repo `shop-agent` has no worktree in workspace `feat-login`: add it with `octok checkout <branch>`
$ octok status
project: shop

repos:
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-backend (backend)    feat/cart  de08597  Fix order totals (5 hours ago)  [not on the remote]
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-agent (agent)        no worktree: add it with `octok checkout feat/cart`
  feat-login  /Users/you/.octotask/workspaces/shop/feat-login
    shop-backend (backend)    feat/login  ec75932  Add login endpoint (3 hours ago)
    shop-frontend (frontend)  feat/login  1695c74  Add login form (4 hours ago)
    shop-agent (agent)        no worktree: add it with `octok checkout feat/login`
```

## Pull requests

| Message | Code, exit | Fix |
|---|---|---|
| `nothing to review ...` | `nothing_to_review`, 2 | Without a terminal, give at least one `--pr <repo>=<number>` or `--branch <repo>=<branch>`. |
| ``repo `shop-backend` is given more than once ...`` | `repo_chosen_twice`, 2 | Keep one pull request or one branch per repository. |
| ``repo `shop-agent` has no branch `nope` on its remote`` | `branch_not_on_remote`, 3 | Push the branch, or fix its name. A `--pr` whose branch was deleted gets it too. |
| ``workspace `r1` already reviews other branches ...`` | `reviews_other_branches`, 5 | Other choices need another `--name`, or `octok rm r1` first. The same choices again only add the missing checkouts. |
| ``pull request #13 of `shop-backend` comes from a fork ...`` | `fork_pull_request`, 1 | Fetch the fork by hand, or ask the author to push a branch of the repository. |
| `` `gh ...` failed in <dir> ...``, with the `gh` output | `command_failed`, 1 | The output says why: no such pull request, or `gh` not signed in, which `gh auth login` fixes. |
| ``` `gh` not found ...``` | `tool_not_found`, 7 | Install `gh`: only `octok pr review` needs it. |

See [Pull request review](commands/pr-review.md).

```console
$ octok pr review
error: nothing to review: choose a pull request or a branch for at least one repo, with `--pr <repo>=<number>` or `--branch <repo>=<branch>`, or by answering `octok pr review` at a terminal
$ octok pr review --pr backend=12 --branch backend=feat/login
error: repo `shop-backend` is given more than once: choose one pull request or one branch for it
$ octok pr review --branch agent=nope
error: repo `shop-agent` has no branch `nope` on its remote
$ octok pr review --pr backend=99
error: `gh pr view 99 --repo git@github.com:acme/shop-backend.git --json number,title,headRefName,author,isCrossRepository` failed in /Users/you/.octotask (exit status: 1)
GraphQL: Could not resolve to a PullRequest with the number of 99. (repository.pullRequest)
$ octok pr review --branch agent=fix/prompt --name r1
created workspace `r1` in shop
  branch from the remote: shop-agent
  new branch from the base: shop-backend, shop-frontend
/Users/you/.octotask/workspaces/shop/r1
$ octok pr review --branch frontend=main --name r1
error: workspace `r1` already reviews other branches: shop-agent=fix/prompt
remove it with `octok rm r1`, or choose another name with `--name`
$ octok pr review --branch agent=fix/prompt --name r3
error: branch `fix/prompt` of `shop-agent` is already checked out in /Users/you/.octotask/workspaces/shop/r1/shop-agent
a branch lives in one workspace at a time: use that workspace, or remove it first
```

## Removing

| Message | Code, exit | Fix |
|---|---|---|
| ``workspace `feat-cart` has work that would be lost:``, then one line per finding | `workspace_has_work`, 4 | Commit and push what you keep, then `octok rm` again. `--force` discards the work: use it only when it is merged or not needed. |
| ``repo `shop-agent` is used by workspaces: r1`` | `repo_in_use`, 5 | Remove those workspaces first with `octok rm <branch>`. |
| ``project `shop` still has workspaces ...`` | `project_in_use`, 5 | Remove them with `octok rm <branch>`, or with their work by `octok project rm --force shop`. |

A finding is `<repo>: uncommitted changes`, `<repo>: N unpushed commits on <branch>`, `<repo>: the worktree cannot be read`, or `other files in the workspace: <names>`.
Run `git status` in a checkout to see its files.

```console
$ echo 'def cart(): ...' > "$HOME/.octotask/workspaces/shop/feat-cart/shop-backend/cart.py"
$ git -C "$HOME/.octotask/workspaces/shop/feat-cart/shop-backend" add cart.py
$ git -C "$HOME/.octotask/workspaces/shop/feat-cart/shop-backend" commit --quiet -m "Add the cart endpoint"
$ octok rm feat/cart
error: workspace `feat-cart` has work that would be lost:
  shop-backend: 1 unpushed commit on `feat/cart`
remove it anyway with `octok rm --force <branch>`
$ octok repo rm agent
error: repo `shop-agent` is used by workspaces: r1
$ octok project rm shop
error: project `shop` still has workspaces: feat-cart, feat-login, r1
remove them first with `octok rm <branch>`
$ octok rm --force feat/cart
removed workspace `feat-cart` from shop
```

## Dev changes

| Message | Code, exit | Fix |
|---|---|---|
| ``the dev changes of `shop-backend` no longer apply to: main.py`` | `stale_dev_changes`, 5 | The branch changed the same lines or lacks the file, and nothing was created. `--allow-stale-dev` creates the workspace with the files as in git: redo the dev changes, then `octok dev save`. |
| ``no checkout has the file `nope` `` | `dev_file_not_found`, 3 | Give `--file` a path relative to the root of a checkout. |
| git refuses to stage a file with dev changes | from git | You changed the lines of a dev change: if it is a new one, run `octok dev save` again. |

See [Dev changes](commands/dev.md).

```console
$ cd "$HOME/.octotask/workspaces/shop/feat-login/shop-backend"
$ sed 's/8000/9005/' main.py > main.new
$ mv main.new main.py
$ octok dev save
shop-backend (backend)
  changed  main.py
shop-frontend (frontend)
  no dev changes
$ cd
$ git clone --quiet git@github.com:acme/shop-backend.git mate
$ git -C mate switch --quiet staging
$ sed 's/8000/7000/' mate/main.py > mate/main.new
$ mv mate/main.new mate/main.py
$ git -C mate commit --quiet -am "Change the port"
$ git -C mate push --quiet
$ octok branch feat/y
error: the dev changes of `shop-backend` no longer apply to: main.py
The branch changed the same lines, or lacks those files. `--allow-stale-dev` creates the workspace with those files as they are in git: make the dev changes again there, then run `octok dev save`.
$ octok branch feat/y --allow-stale-dev
created workspace `feat-y` in shop
/Users/you/.octotask/workspaces/shop/feat-y
$ octok dev save --file nope --branch feat/login
error: no checkout has the file `nope`
```

## Other machines

| Message | Code, exit | Fix |
|---|---|---|
| ``cannot run commands on `<host>` over ssh``, then the `ssh` output and hints | `ssh`, 7 | Run `ssh <host>` yourself, and answer a passphrase or host key prompt once, or run `octok set-ssh-passphrase <host>`. The `ssh` text names the cause: `Connection refused` or `Connection timed out` (network), `Permission denied (publickey)` (key), `Host key verification failed` (`known_hosts`). |
| ``` `octok set-ssh-passphrase` asks at the terminal ...``` | `needs_terminal`, 2 | Run it yourself at a terminal, without `--json`. |
| ``` `<program>` not found: install it or add it to PATH``` | `tool_not_found`, 7 | Put `git`, `gh`, `ssh` or `ssh-add` on `PATH`. `ssh` is needed only for another machine, `gh` only for `octok pr review`. |
| ``cannot run `<program>` in <dir>: <reason>`` | `spawn`, 7 | Check that the folder exists and that you may run the program. |

The checklist is in [Repositories on another machine](guides/remote-machine.md).

```console
$ octok repo add git@github.com:acme/shop-agent.git --name agent2 --location unreachable
error: cannot run commands on `unreachable` over ssh
ssh: connect to host unreachable port 22: Connection refused
check that `ssh unreachable` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`
if its key has a passphrase, run `octok set-ssh-passphrase unreachable`
$ octok set-ssh-passphrase --json
{"schema":1,"error":{"code":"needs_terminal","message":"`octok set-ssh-passphrase` asks at the terminal, which `--json` never does: run it without `--json`","command":"octok set-ssh-passphrase"}}
```

## When some repositories fail

`octok fetch` and `octok pull` keep going when a repository fails: they print `failed` and its error, and exit with 6.
With `--json` the normal document has an `error` in each failed entry of `repos`, and none at the top.

| Message | Code, exit | Fix |
|---|---|---|
| ``` `<command>` failed in <dir> (<status>)```, then the program output | `command_failed`, 1 | `fatal: Could not read from remote repository`: your credentials do not reach the remote. `does not appear to be a git repository`: fix the URL with `octok repo set <repo> --url <url>`. |
| ``` `<command>` printed `<output>`, which octok does not understand``` | `unexpected_output`, 1 | An unexpected version of `git`, `gh` or `ssh`, or a remote shell that prints text on login. Run the command by hand, and report it if the output is plain. |

```console
$ env PATH=/nonexistent "$(command -v octok)" fetch
shop-backend (backend)    failed
error: shop-backend (backend): `git` not found: install it or add it to PATH
shop-frontend (frontend)  failed
error: shop-frontend (frontend): `git` not found: install it or add it to PATH
shop-agent (agent)        failed
error: shop-agent (agent): `git` not found: install it or add it to PATH
```

`octok pull` leaves a checkout where it is, without an error, for these lines:

| Line | Fix |
|---|---|
| `skipped: branch is not on the remote yet` | Only here, or deleted after a merge: `git push` in the checkout if you want it there. |
| `skipped: diverged from the remote branch` | `octok pull` never merges, rebases or resets: run `git merge` or `git rebase` in the checkout. |
| `skipped: detached HEAD` | `git switch <branch>`. |
| `skipped: no worktree in this workspace` | `octok checkout <branch>`. |

## Reading `octok status`

`octok status` reports the remotes as of the last fetch, and [Status](commands/status.md) explains each of its marks.

| Shown | Fix |
|---|---|
| ``not fetched (run `octok fetch`)`` | Run `octok fetch`. |
| ``branch `<base>` is not on the remote`` | Push the base branch, or change it with `octok project set --base`. |
| ``no worktree: add it with `octok checkout <branch>` `` | Run `octok checkout <branch>`. |
| `cannot be read: <reason>` | The folder is missing or git cannot read it: `octok checkout <branch>` restores a missing one, with its commits. |
| `orphan caches (not in the project file):` | Clones of repositories no longer in the project: safe to delete. |
| `[not on the remote]` | Nothing: a new branch is so until you push it. |

## Project files

These errors name the path of a [project file](reference/configuration.md) that `octok` cannot use.

| Message | Code, exit | Fix |
|---|---|---|
| `<path>: unsupported config version 9 (expected 1)` | `unsupported_version`, 1 | Use the version of `octok` that wrote it, or update `octok`. |
| ``<path>: file declares project `other` but is named after `mismatch` `` | `name_mismatch`, 1 | Make `name` match the file name. |
| `<path>: <message>`, such as `expected string scalar at line 1, column 10` | `parse`, 1 | Fix the YAML or the missing field at that line and column. The message never quotes the file, which can hold secret env values. |
| `<path>: cannot serialize: <message>` | `serialize`, 1 | `octok` could not write a file: not expected in normal use. |
| `<path>: <reason>`, such as `Permission denied (os error 13)` | `io`, 1 | Check the rights and the free space of the path. |
| `cannot locate home directory (set OCTOTASK_HOME)` | `no_home`, 1 | Set `OCTOTASK_HOME`, which also moves everything `octok` keeps. |

```console
$ printf 'version: 9\nname: old\n' > "$HOME/.octotask/projects/old.yaml"
$ octok -p old status
error: /Users/you/.octotask/projects/old.yaml: unsupported config version 9 (expected 1)
$ printf 'version: 1\nname: other\nbase: staging\nrepos: {}\n' > "$HOME/.octotask/projects/mismatch.yaml"
$ octok -p mismatch status
error: /Users/you/.octotask/projects/mismatch.yaml: file declares project `other` but is named after `mismatch`
```

## Behaviors that are not errors

### A workspace that needs `--force`

`octok rm` counts as unpushed every commit that no remote branch had at the last fetch.
After a squash merge and the deletion of its branch, your own commits are on no remote branch, so check that the work is merged, for example with `git log origin/staging`, then use `--force`.
Ignored files such as `.env` and the dev changes go without a warning: the dev changes are saved elsewhere.

### A branch is reported as not pushed

`[not on the remote]`, `skipped: branch is not on the remote yet` and `unpushed commit` describe the last fetch, so they hold for a branch never pushed and for one deleted after its merge.
If you pushed from another machine, run `octok fetch` first.

### `upstream is gone`

When the remote branch is deleted, `octok fetch` and `octok pull` prune it, and `git status` in the checkout shows `[gone]` after the upstream:

```console
$ git -C "$HOME/mate" push --quiet origin --delete feat/login
$ octok fetch
shop-backend (backend)    fetched
shop-frontend (frontend)  fetched
shop-agent (agent)        fetched
$ git -C "$HOME/.octotask/workspaces/shop/feat-login/shop-backend" status --short --branch
## feat/login...origin/feat/login [gone]
$ octok status
project: shop

repos:
  shop-backend (backend)    staging  640a73a  Change the port (10 minutes ago)
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)

workspaces:
  feat-login  /Users/you/.octotask/workspaces/shop/feat-login
    shop-backend (backend)    feat/login  ec75932  Add login endpoint (3 hours ago)  [not on the remote]
    shop-frontend (frontend)  feat/login  1695c74  Add login form (4 hours ago)
    shop-agent (agent)        no worktree: add it with `octok checkout feat/login`
  feat-y  /Users/you/.octotask/workspaces/shop/feat-y
    shop-backend (backend)    feat/y  640a73a  Change the port (10 minutes ago)  [not on the remote]
    shop-frontend (frontend)  feat/y  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-agent (agent)        feat/y  a1ea388  Tune the prompt (3 days ago)  [not on the remote]
  r1  /Users/you/.octotask/workspaces/shop/r1
    shop-backend (backend)    r1          de08597  Fix order totals (5 hours ago)  [not on the remote]
    shop-frontend (frontend)  r1          1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-agent (agent)        fix/prompt  1967eea  Shorten the prompt (6 hours ago)
```

Your branch and its commits stay: push it again if you need it, or remove the workspace once the work is merged.

### `setup` does not run again

A setup command runs until it succeeds once in a checkout, and again when its text changes: change it with `octok repo set <repo> --setup <command>`, even by a space, to run it again.

### `octok checkout` created a workspace

`octok checkout <branch>` creates the workspace whenever a remote has the branch, even one you just removed: use `octok status` to look without creating.

### ssh asks for a passphrase

A key with a passphrase makes `ssh` ask at every command, and without a terminal `octok` fails at once with the `ssh` error.
Run `octok set-ssh-passphrase <host>` once, and the ssh agent keeps it.
