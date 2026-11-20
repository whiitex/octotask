# Troubleshooting

Every error that `octok` reports, grouped by the situation you meet it in, with the message as printed, what it means and what to do, followed by the behaviors that look like errors and are not.

## Reading an error

A command that fails prints `error: <message>` on stderr and exits with a code other than 0.
The code says what kind of failure it was, and it is the same with and without `--json`:

| Exit | Meaning |
|---|---|
| 0 | The command did what it was asked. |
| 1 | Another error: git or `gh` failed, a file cannot be read. |
| 2 | The command is not written the way `octok` takes it: an unknown option, a name that is not valid, no project given. |
| 3 | Something the command names is not there: a project, a repository, a branch, a workspace, a run command, a page of the documentation. |
| 4 | Refused, because it would lose work that exists nowhere else. |
| 5 | Refused, because it conflicts with what exists: already there, in use, or no longer applies. |
| 6 | The command went through the repositories one by one, and at least one failed. |
| 7 | Another machine or a program cannot be reached. |
| 130 | `octok up` was stopped with Ctrl-C or a signal. |

A shell does not show the exit code by itself: `octok status || echo "exit $?"` prints it.
With `--json` the failure is a document on stdout with a stable `code`, listed in [JSON output](agents/json-output.md) next to the other fields of each error.
Every error below gives its `code` and its exit code, so a script can decide on them and not on the text.

When you write `cd "$(octok branch feat/cart)"` and the command fails, the message is shown and `cd` receives nothing: you stay in the folder you were in.
Read the message before you go on typing.

The sections below are in the order a project grows.
The examples use the project `shop` and its fixture repositories, and each block continues from the one before it.

## Starting out: projects and names

Most commands act on a project, so they fail until one exists: in a clean home only `octok project init`, `octok project ls`, `octok docs` and `--help` work.
These commands fail there:

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

- `` no project given and no default project set (use `octok project use <name>`) `` (`no_default_project`, exit 2): no project is the default.
  Create one with `octok project init <name>`, choose one with `octok project use <name>`, or pass `-p <name>` before or after the command.
- `` project `nope` not found `` (`project_not_found`, exit 3): there is no project with that name.
  `octok project ls` lists the ones that exist.
- `` topic `nope` not found: list the topics with `octok docs` `` (`topic_not_found`, exit 3): a page of the documentation that does not exist.
  `octok docs` lists the pages with their topic.
- A message that starts with `error: unrecognized subcommand`, `unexpected argument` or `the following required arguments were not provided` is the command line itself being refused, with the usage under it (`usage`, exit 2).
  Run the command with `--help`.
  With `--json` this also is a document, whatever `--json` is next to.

Names have rules, because they become folders and branches:

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

- `` invalid <kind> `<value>`: <reason> `` (`invalid_name`, exit 2): `<kind>` is `project name`, `repo key`, `alias`, `branch`, `workspace name`, `file`, or the name of a command such as `run command`.
  The reason says what is wrong.
  Project names and keys have at most 64 characters, start with a letter or a digit and contain only letters, digits, `.`, `_` and `-`.
  A branch name must be one git accepts, and must not start with `-`.
  The `--name` of `octok pr review` follows the rules of a branch, because the repositories you do not choose get a branch with that name.
  `workspace name` comes only from a project file edited by hand: the name of a workspace has no `/`.
  A command cannot be empty.
- `` invalid location `<value>`: <reason> `` (`invalid_location`, exit 2): the host of a [location](concepts.md#location) may only contain letters, digits, `.`, `_`, `-` and `@`, must not start with `-` and must not be empty.
  In a project file, a location must be `local` or `<host>:<absolute path>`.
- `` project `shop` already exists `` (`project_exists`, exit 5): choose another name, or `octok project use shop`.
- `` project `shop` has no repos: add one with `octok repo add <url>` `` (`no_repos`, exit 3): the commands that act on every repository need at least one.

## Registering repositories

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

- `` invalid git url `<url>` `` (`invalid_url`, exit 2): the text is not one of the forms `octok` takes: the scp-like `git@github.com:acme/shop-backend.git`, an `https://`, `http://`, `ssh://`, `git://` or `file://` URL, or an absolute path, with no spaces.
  A relative path is refused: write it from `/`.
- `` cannot derive a repo key from `<url>`: choose one with `--name <key>` `` (`no_repo_key`, exit 2): the last part of the URL gives no name.
  Choose the key with `--name`.
- `` repo `shop-backend` is already registered (<url>) `` followed by `` choose another name with `--name <key>` `` (`duplicate_repo`, exit 5): two repositories cannot have the same key.
  This happens when two repositories have the same name, or when you register one twice.
  Use `--name` for the second one.
- `` alias `backend` is already used by repo `shop-backend` `` (`duplicate_alias`, exit 5): an alias is accepted wherever a key is, so it must be unique.
- `` repo `nope` is not registered `` (`repo_not_found`, exit 3): a key or an alias that no repository has.
  `octok repo ls` lists them.
- `` repo `shop-backend` has no branch `nope` on its remote `` (`base_not_found`, exit 3): the base branch you chose is not on the remote.
  At a terminal `octok` asks whether to use it anyway, and without a terminal, with `--json` or in a script, it refuses with this error.
  Check the name with `git ls-remote --heads <url>`.
- `` warning: branch `nope` is not on the remote of: ... `` is not an error: `octok project set --base` saved the new base and warns about the repositories that do not have it.
  `octok status` keeps saying so, as `` branch `nope` is not on the remote ``, until you change the base back or the branch exists.

## Creating and finding workspaces

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

- `` no repo has a branch `nope` on its remote: create it with `octok checkout -b nope` `` (`branch_not_found`, exit 3): `octok checkout` is for a branch that someone pushed.
  For a new branch use `octok branch <branch>`.
- `` branch `feat/login` already exists on the remote of: ... `` followed by `` check it out with `octok checkout feat/login`, or choose another name `` (`branch_exists`, exit 5): `octok branch` never lands on a branch that exists.
  Use `octok checkout`, or another name.
- `` workspace `feat-cart` already exists at <path> `` (`workspace_exists`, exit 5): the workspace of that branch is there already.
  `cd "$(octok checkout feat/cart)"` goes into it, and adds the checkouts it lacks.
- `` workspace `feat-cart` is the one of the branch `feat/cart`, whose name gives the same folder `` (`workspace_of_another_branch`, exit 5): `feat/cart` and `feat-cart` are both the workspace `feat-cart`, and only one of them can have it.
  Remove the other one, or use another branch name.
- `` no workspace for `feat-nope`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch `` (`workspace_not_found`, exit 3): there is no workspace for that branch.
  `octok rm` names the branch you gave, `feat/nope`, and the other commands the name of the workspace, `feat-nope`; the `workspace` of the JSON error is always the name.
  `octok status` lists the ones that exist.
  The commands that take a branch take the workspace of that branch, never a folder name of your own.
- `` not inside a workspace: pass its branch, as in `octok pull <branch>` `` (`not_in_workspace`, exit 2): you left out the branch and the current folder is not inside a workspace.
  Pass the branch, or `cd` into the workspace.
- `` branch `fix/prompt` of `shop-agent` is already checked out in <path> ``, followed by `a branch lives in one workspace at a time: use that workspace, or remove it first` (`branch_in_use`, exit 5): git keeps a branch in one checkout at a time.
  The path is the checkout that holds it.

## Running

```console
$ octok up --branch feat/login
error: no repo of workspace `feat-login` has a `run` command: set one with `octok repo set <repo> --run <command>`
$ octok up backend --branch feat/login
error: repo `shop-backend` has no `run` command: set one with `octok repo set shop-backend --run <command>`
```

- `` no repo of workspace `feat-login` has a `run` command: set one with `octok repo set <repo> --run <command>` `` (`nothing_to_run`, exit 3): no repository of the workspace says how it starts.
- `` repo `shop-backend` has no `run` command: set one with `octok repo set shop-backend --run <command>` `` (`no_run_command`, exit 3): you named a repository without a run command.
- `repo ... has no worktree in workspace ...` is in the next section.

When `octok up` runs, the line `<repo> | ended (exit status: N)` is how a repository ended, and `octok up` exits with 1 when any of them ended with a failure and 130 when you stopped it.
A repository whose setup command failed is not started: read the output above its `$ <setup command>` line.

## A repository added later

A repository registered after a workspace was created has no checkout in it until you add one:

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

- `` repo `shop-agent` has no worktree in workspace `feat-login`: add it with `octok checkout <branch>` `` (`no_worktree`, exit 3): run `octok checkout feat/login`, which adds the checkouts the workspace lacks.
  `octok status` shows the same as `` no worktree: add it with `octok checkout <branch>` ``.

## Pull requests

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

- `` nothing to review: choose a pull request or a branch for at least one repo, with `--pr <repo>=<number>` or `--branch <repo>=<branch>`, or by answering `octok pr review` at a terminal `` (`nothing_to_review`, exit 2): without a terminal at least one `--pr` or `--branch` is needed.
- `` repo `shop-backend` is given more than once: choose one pull request or one branch for it `` (`repo_chosen_twice`, exit 2): keep one choice per repository.
- `` repo `shop-agent` has no branch `nope` on its remote `` (`branch_not_on_remote`, exit 3): the branch is not pushed, or the name is misspelled.
  A `--pr` whose branch was deleted gets it too.
- `` workspace `r1` already reviews other branches: shop-agent=fix/prompt `` followed by `` remove it with `octok rm r1`, or choose another name with `--name` `` (`reviews_other_branches`, exit 5): a review workspace remembers its choices.
  Other choices need another workspace.
  The same choices again only add the checkouts it lacks.
- The pull request of a fork (`fork_pull_request`, exit 1) is refused, because its branch is not in the repository itself:

  ```text
  error: pull request #13 of `shop-backend` comes from a fork: octok can only review a branch of the repo itself
  ```

  Fetch the fork by hand, or ask the author to push a branch of the repository.
- `gh` failures are reported as `command_failed`, with what `gh` said under the summary.
  A pull request that does not exist is the first line of the output above.
  A `gh` that is not signed in says so there, and `gh auth login` fixes it.
  Without `gh` installed, the error is ``error: `gh` not found: install it or add it to PATH`` (`tool_not_found`, exit 7).
  The other `octok` commands do not need `gh`.

## Removing

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

- `` workspace `feat-cart` has work that would be lost: `` followed by one line per repository (`workspace_has_work`, exit 4): the workspace holds uncommitted changes, unpushed commits or files that do not belong to a checkout.
  Commit and push what you want to keep, then run `octok rm` again.
  `--force` deletes the workspace and discards that work, so run it only when you know it is merged or no longer needed.
  Each line is one finding: `<repo>: uncommitted changes`, `<repo>: N unpushed commits on <branch>`, `<repo>: the worktree cannot be read`, or `other files in the workspace: <names>` for what sits in the workspace folder next to the checkouts.
  A line about a repository does not name the files: run `git status` in that checkout.
- `` repo `shop-agent` is used by workspaces: r1 `` (`repo_in_use`, exit 5): remove those workspaces first with `octok rm <branch>`.
- `` project `shop` still has workspaces: ... `` followed by `` remove them first with `octok rm <branch>` `` (`project_in_use`, exit 5): the same for `octok project rm`.
  `octok project rm --force shop` deletes them with the work they hold.

## Dev changes

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

- `` the dev changes of `shop-backend` no longer apply to: main.py `` (`stale_dev_changes`, exit 5): the branch changed the very lines of a dev change, or lacks the file, so the saved change cannot be put on top of it.
  Nothing was created.
  `--allow-stale-dev` creates the workspace with those files as they are in git: make the dev changes again there and run `octok dev save`.
  See [Dev changes](commands/dev.md).
- `` no checkout has the file `nope` `` (`dev_file_not_found`, exit 3): `--file` names a file that is in no checkout of the workspace.
  The path is relative to the root of each checkout.
- git refuses to stage a file with dev changes, and says so, when you change the very lines of a dev change.
  If that is a new dev change, run `octok dev save` again.

## Other machines

```console
$ octok repo add git@github.com:acme/shop-agent.git --name agent2 --location unreachable
error: cannot run commands on `unreachable` over ssh
ssh: connect to host unreachable port 22: Connection refused
check that `ssh unreachable` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`
if its key has a passphrase, run `octok set-ssh-passphrase unreachable`
$ octok set-ssh-passphrase --json
{"schema":1,"error":{"code":"needs_terminal","message":"`octok set-ssh-passphrase` asks at the terminal, which `--json` never does: run it without `--json`","command":"octok set-ssh-passphrase"}}
```

- ``cannot run commands on `<host>` over ssh`` followed by the output of `ssh` and two hints (`ssh`, exit 7): `ssh` did not connect, or a command on the other machine could not start.
  Run `ssh <host>` yourself first.
  If it asks for a passphrase or a host key, answer once, or run `octok set-ssh-passphrase <host>`.
  The text of `ssh` says whether it was the network (`Connection refused`, `Connection timed out`), the key (`Permission denied (publickey)`) or `known_hosts` (`Host key verification failed`).
  The whole checklist is in [Repositories on another machine](guides/remote-machine.md#when-the-connection-fails).
- `` `octok set-ssh-passphrase` asks at the terminal, which `--json` never does: run it without `--json` `` (`needs_terminal`, exit 2): the command only works by asking.
  Run it yourself at a terminal.
- ``` `ssh` not found: install it or add it to PATH ``` (`tool_not_found`, exit 7): a program that `octok` runs is not on `PATH`.
  It can be `git`, `gh`, `ssh` or `ssh-add`.
  `ssh` is only needed for repositories on another machine, and `gh` only for `octok pr review`.
- ``` cannot run `<program>` in <dir>: <reason> ``` (`spawn`, exit 7): the program exists but could not start, for example because the folder it should run in is missing.
  Check that the folder exists and that you may run the program.
- `ssh` asking for a passphrase at every command, and `octok` failing at once without a terminal, are the same thing: see [The ssh passphrase](guides/remote-machine.md#the-ssh-passphrase).

## When some repositories fail

`octok fetch` and `octok pull` go through the repositories one by one and keep going when one fails.
They print `failed` for it, the error after, and exit with 6.
Here `git` is not on `PATH`, so every repository fails:

```console
$ env PATH=/nonexistent "$(command -v octok)" fetch
shop-backend (backend)    failed
error: shop-backend (backend): `git` not found: install it or add it to PATH
shop-frontend (frontend)  failed
error: shop-frontend (frontend): `git` not found: install it or add it to PATH
shop-agent (agent)        failed
error: shop-agent (agent): `git` not found: install it or add it to PATH
```

With `--json`, the document is the normal one, with an `error` in the entry of each failed repository and no `error` at the top: read `repos`.

- `` `<command>` failed in <dir> (<status>) `` followed by the output of the program (`command_failed`, exit 1): a `git`, `gh` or `ssh` command failed.
  The output under the line is the program's own: `fatal: Could not read from remote repository` means the remote is not reachable with your credentials, `does not appear to be a git repository` means the URL is wrong (change it with `octok repo set <repo> --url <url>`).
- `` `<command>` printed `<output>`, which octok does not understand `` (`unexpected_output`, exit 1): a program printed something `octok` could not read, which is a different version of `git`, `gh` or `ssh` than the one `octok` expects, or a shell that prints text on login on the other machine.
  Run the command it names by hand, and report it if the output is a plain one.

The lines of `octok pull` for a repository that was not fast-forwarded are not errors, but the checkout did not move:

- `updated` and `up to date` need nothing.
- `skipped: branch is not on the remote yet`: the branch exists only here, or was deleted on the remote after a merge.
  Push it with `git push` in that checkout if you want it there.
- `skipped: diverged from the remote branch`: your commits and the remote ones differ.
  `octok pull` never merges, rebases or resets: bring the checkout in line with `git merge` or `git rebase` in it.
- `skipped: detached HEAD`: the checkout is on a commit and not on a branch.
  Switch to the branch with `git switch <branch>`.
- `skipped: no worktree in this workspace`: the repository has no checkout here: `octok checkout <branch>` adds it.

## Reading `octok status`

`octok status` does not contact the git remotes: what it says about them is from the last fetch.
It names the problems it finds:

- `` not fetched (run `octok fetch`) `` instead of a commit: the repository was never fetched.
- `` branch `<base>` is not on the remote `` instead of a commit: the base branch of the repository did not exist at the last fetch.
- `` no worktree: add it with `octok checkout <branch>` `` in a workspace: the repository has no checkout there.
- `cannot be read: <reason>` in a checkout: its folder is missing or git cannot read it.
  `octok checkout <branch>` restores a missing one, with its commits.
- `(detached)` in place of the branch: the checkout is on a commit and not on a branch, and `octok pull` skips it.
- `orphan caches (not in the project file):` and a list of folders: clones left behind by repositories that are no longer in the project.
  They are safe to delete.
- `[not on the remote]` after a checkout: the branch was not on the remote at the last fetch.
  This is a note, not a problem: a new branch is in that state until you push it.

## Project files

`octok` keeps a project in `~/.octotask/projects/<name>.yaml`: see [Configuration files](reference/configuration.md).
Editing it by hand is possible, and a file that `octok` cannot use is reported with its path:

```console
$ printf 'version: 9\nname: old\n' > "$HOME/.octotask/projects/old.yaml"
$ octok -p old status
error: /Users/you/.octotask/projects/old.yaml: unsupported config version 9 (expected 1)
$ printf 'version: 1\nname: other\nbase: staging\nrepos: {}\n' > "$HOME/.octotask/projects/mismatch.yaml"
$ octok -p mismatch status
error: /Users/you/.octotask/projects/mismatch.yaml: file declares project `other` but is named after `mismatch`
```

- `<path>: unsupported config version 9 (expected 1)` (`unsupported_version`, exit 1): the file was written by another version of `octok`.
  Use the version that wrote it, or update `octok`.
- `` <path>: file declares project `other` but is named after `mismatch` `` (`name_mismatch`, exit 1): the `name` in the file and the name of the file must be the same.
- `<path>: <message>` (`parse`, exit 1): the file is not valid YAML, or lacks a field.
  The message of the parser, with the line and the column, follows:

  ```text
  error: /Users/you/.octotask/projects/broken.yaml: error: line 1 column 10: expected string scalar
   --> <input>:1:10
    |
  1 | version: [
    |          ^ expected string scalar
  ```
- `<path>: cannot serialize: <message>` (`serialize`, exit 1): `octok` could not write a file.
  It is not expected in normal use.
- `<path>: <reason>` with a system reason such as `Permission denied (os error 13)` or `No space left on device` (`io`, exit 1): a file or a folder could not be read or written.
  Check the rights and the space of the path it names.
- `cannot locate home directory (set OCTOTASK_HOME)` (`no_home`, exit 1): `octok` found no home folder.
  Set `OCTOTASK_HOME`, which also moves everything `octok` keeps.

## Behaviors that are not errors

### A workspace that needs `--force`

`octok rm` refuses when it finds work that exists nowhere else, and `--force` discards it.
It counts as unpushed every commit of the branch that no branch of the remote has, as of the last fetch.
After a squash merge the base branch has a new commit with the same changes, and once the branch of the pull request is deleted on the remote, no remote branch has your own commits.
`octok rm` then reports them as unpushed.
Check that the work is merged, for example with `git log origin/staging` in the checkout, and then use `--force`.
Files that git ignores, such as `.env`, go without a warning with the workspace, and so do the dev changes: they are saved elsewhere.

### A branch is reported as not pushed

`[not on the remote]` in `octok status` and `skipped: branch is not on the remote yet` in `octok pull` mean that the remote had no branch with that name at the last fetch.
`unpushed commit` in `octok rm` means that no branch of the remote had those commits at the last fetch.
Both are true for a branch you never pushed, and for a branch deleted after its pull request was merged.
`octok` compares with what the last fetch saw, so `octok fetch` first if you pushed from another machine.

### `upstream is gone`

When the remote branch is deleted, `octok fetch` and `octok pull` prune it, and git says that the upstream of your branch is gone:

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

Your local branch and its commits are untouched.
Push the branch again if you still need it, or remove the workspace once the work is merged.

### `setup` does not run again

The setup command of a repository runs before its run command until it succeeds once in that checkout, and again when its text changes.
A second `octok up` does not run `npm ci` again.
Change the command with `octok repo set <repo> --setup <command>`, even by a space, to make it run again.
When it fails, the repository is not started.

### `octok checkout` created a workspace

`octok checkout <branch>` creates the workspace whenever a remote has the branch, even one you removed a moment ago.
To look without creating, use `octok status`.

### ssh asks for a passphrase

A key with a passphrase makes `ssh` ask at every command.
Run `octok set-ssh-passphrase <host>` once, and the ssh agent keeps it.
Without a terminal `octok` never waits for it: it fails with the `ssh` error above.
