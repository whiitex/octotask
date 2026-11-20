# Pull request review

How `octok pr review` builds one workspace with each repository on a pull request or a branch you choose, and what it needs from `gh`.

## What it is for

A change often spans pull requests with different branch names: one in the backend, another in the agent, nothing in the frontend.
`octok pr review` builds one [workspace](../concepts.md#workspace) for all of them.
Each repository you name is on the branch of its pull request, or on the branch you give.
Every other repository starts from its [base branch](../concepts.md#base-branch), so the workspace is complete and you can run all of it with [`octok up`](up.md).
The command prints the path of the workspace.

```text
octok pr review [--pr <repo>=<number>]... [--branch <repo>=<branch>]... [--name <name>] [--location <location>] [--allow-stale-dev]
```

`<repo>` is the key or the alias of a repository of the project.
See the [command reference](../reference/commands.md#octok-pr-review) for the generated list of options.

## The usual use

The project below has three repositories.
Pull request #12 of the backend is the branch `feat/login`, and the agent has a branch `fix/prompt` with no pull request.

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo add git@github.com:acme/shop-agent.git --alias agent
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
$ cd "$(octok pr review --pr backend=12 --branch agent=fix/prompt)"
shop-backend (backend): pull request #12 is the branch `feat/login`
created workspace `pr-backend-12` in shop
  branch from the remote: shop-backend, shop-agent
  new branch from the base: shop-frontend
$ ls
shop-agent
shop-backend
shop-frontend
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)

workspaces:
  pr-backend-12  /Users/you/.octotask/workspaces/shop/pr-backend-12
    shop-frontend (frontend)  pr-backend-12  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/login     ec75932  Add login endpoint (3 hours ago)
    shop-agent (agent)        fix/prompt     1967eea  Shorten the prompt (6 hours ago)
```

The line about the pull request goes to stderr, and the path of the workspace goes to stdout, so `cd "$(...)"` enters the workspace and still shows the line.
When you are done, remove the workspace with [`octok rm pr-backend-12`](workspaces.md#delete-a-workspace).
It also deletes the local branches that the workspace checked out, unless another workspace has them.

## What each repository gets

- A repository with `--pr` or `--branch` gets a local branch at the commit of the remote branch, and that branch follows the remote one.
  `git pull`, `git push` and [`octok pull`](sync.md) work on it.
- Any other repository gets a new branch from its up-to-date base branch, named after the workspace, so the workspace is complete.
  In the example, that is `pr-backend-12` in `shop-frontend`.
  The branch exists only on your machine until you push it.
  If the remote of that repository already has a branch with that name, it gets that branch instead.
- The command fetches the repositories first, so a branch is read as it is on the remote now.
- The message `created workspace` lists the repositories by where their branch came from: `branch from the remote` and `new branch from the base`.

## The options

### `--pr <repo>=<number>`

Reviews the pull request `<number>` of `<repo>`.
`12` and `#12` both work after the `=`.
`octok` asks `gh` for the branch of that pull request, whatever its state: open, closed or merged.
It prints `` <repo>: pull request #12 is the branch `feat/login` `` on stderr before it creates anything, with the repository shown as in the example above.
You can repeat the option for other repositories.

### `--branch <repo>=<branch>`

Reviews the branch `<branch>` of `<repo>`, which must be on the remote of that repository.
It never calls `gh`, so it works for a branch with no pull request, and when `gh` is not installed.
You can repeat the option for other repositories.
A branch whose name is only digits needs `--branch`, because the picker below reads a number as a pull request.

A repository can be given once: `--pr backend=12 --branch backend=feat/login` is refused.

### `--name <name>`

Names the workspace and the branch of the repositories left on their base.
The name is checked like a branch name, and a workspace is named after a branch with `/` replaced by `-`: see [workspaces](workspaces.md).

Without `--name`, the workspace is named `pr-<repo>-<number>` after the first pull request, in the order of the repositories of the project, using the alias of the repository when it has one.
With only `--branch`, it is named after the first of those branches, in the same order.
So `--branch agent=fix/prompt` alone makes the workspace `fix-prompt`, on the branch `fix/prompt` in every repository that has no choice.

Use the name with the other commands: `octok up --branch pr-backend-12`, `octok pull pr-backend-12`, `octok rm pr-backend-12`.

### `--location <location>`

Puts the whole workspace on another machine, as `octok branch --location` does.
See [Locations and SSH](locations.md).

### `--allow-stale-dev`

Creates the workspace even when some of your saved [dev changes](dev.md) no longer apply to the branch a checkout starts from.
Without it, nothing is created and the error names the file.

## At a terminal

Without `--pr` and `--branch`, and at a terminal, `octok` asks what to take for each repository.
It lists the open pull requests of the repository, the 15 newest, and asks:

```text
Open pull requests of shop-backend (backend):
  #12  Add login endpoint  (feat/login, alice)
  #13  Fix typo  (patch-1, alice, from a fork)
Pull request or branch of shop-backend (backend) [none, base `staging`]:
```

- A number such as `12` or `#12` takes that pull request.
- A name takes that branch.
- Enter leaves the repository on its base branch.
- A title is cut to 72 characters and shows no control characters.
- A repository with no open pull request prints `No open pull requests of <repo>.`
- When `gh` cannot list the pull requests of a repository, `octok` prints why and asks anyway: you can still type a branch.
- An answer that cannot be reviewed is explained and asked again, for example a branch that is not on the remote of the repository, or a pull request from a fork.
- If you leave every repository on its base, the command stops with `nothing to review`, exit code 2, and creates nothing.

The questions stop before they start when the command cannot succeed, for example a project with no repositories or a `--name` that is not a valid branch name.
With `--json` or without a terminal nothing is asked: at least one `--pr` or `--branch` is needed.

## What is remembered

A workspace made by `octok pr review` remembers what it reviews in the project file, until [`octok rm`](workspaces.md) removes the workspace.
The checkouts then know their branch, and:

- Running the same command again prints the path of the existing workspace and changes nothing.
- [`octok checkout <name>`](workspaces.md) completes a workspace that lacks some checkouts, such as a repository registered after it was made or a folder you deleted, and puts each one on the branch it had.
- A different choice for a workspace that exists is refused with exit code 5, and so is a `--name` that is already the name of a workspace made by `octok branch` or `octok checkout`.

```console
$ octok pr review --pr backend=12 --branch agent=fix/prompt
shop-backend (backend): pull request #12 is the branch `feat/login`
/Users/you/.octotask/workspaces/shop/pr-backend-12
$ octok pr review --pr backend=12 --branch frontend=feat/login --name pr-backend-12
shop-backend (backend): pull request #12 is the branch `feat/login`
error: workspace `pr-backend-12` already reviews other branches: shop-backend=feat/login, shop-agent=fix/prompt
remove it with `octok rm pr-backend-12`, or choose another name with `--name`
$ rm -rf "$OCTOTASK_HOME/workspaces/shop/pr-backend-12/shop-agent"
$ octok checkout pr-backend-12
completed workspace `pr-backend-12` in shop
  branch restored from the cache: shop-agent
/Users/you/.octotask/workspaces/shop/pr-backend-12
$ git -C "$OCTOTASK_HOME/workspaces/shop/pr-backend-12/shop-agent" branch --show-current
fix/prompt
```

## Pull requests from forks

A pull request from a fork is refused: its branch is not in the repository itself, so `octok` has nothing to check out from the remote it clones.
The message is ``pull request #<number> of `<repo>` comes from a fork: octok can only review a branch of the repo itself``, with exit code 1.
At a terminal, the list marks such a pull request with `from a fork`, and choosing it asks again.

## One branch, one workspace

git keeps a branch in one checkout at a time.
A branch that another workspace already has is refused, with the folder that holds it, and nothing is created:

```text
error: branch `feat/login` of `shop-backend` is already checked out in /Users/you/.octotask/workspaces/shop/feat-login/shop-backend
a branch lives in one workspace at a time: use that workspace, or remove it first
```

This is why `octok pr review --pr backend=12` fails once you have run `octok checkout feat/login`: the workspace `feat-login` already holds the branch.

## What needs `gh`

`--pr` and the list of pull requests at a terminal use the GitHub CLI `gh`, which must be installed and signed in with `gh auth login`.
`octok` runs `gh pr view` for `--pr` and `gh pr list` for the picker, with the URL of the repository, so a repository that is not on GitHub or a `gh` that is not signed in fails with the message of `gh`.
If `gh` is missing, the error is `` `gh` not found: install it or add it to PATH `` with exit code 7.
`gh` always runs on this machine, also for a repository that lives on [another machine](locations.md).
Nothing else in `octok` needs `gh`, and `--branch` does not.

## What is refused

The refusals happen before anything is created, so a failed command leaves nothing behind.

```console
$ octok pr review
error: nothing to review: choose a pull request or a branch for at least one repo, with `--pr <repo>=<number>` or `--branch <repo>=<branch>`, or by answering `octok pr review` at a terminal
$ octok pr review --pr backend=12 --branch backend=feat/login
error: repo `shop-backend` is given more than once: choose one pull request or one branch for it
$ octok pr review --pr backend=nope
error: invalid value 'backend=nope' for '--pr <REPO=NUMBER>': expected `<repo>=<number>`, e.g. `backend=12`

For more information, try '--help'.
$ octok pr review --branch agent=feat/nope
error: repo `shop-agent` has no branch `feat/nope` on its remote
$ octok pr review --pr nothing=1
error: repo `nothing` is not registered
$ octok pr review --pr backend=99
error: `gh pr view 99 --repo git@github.com:acme/shop-backend.git --json number,title,headRefName,author,isCrossRepository` failed in /Users/you/.octotask (exit status: 1)
GraphQL: Could not resolve to a PullRequest with the number of 99. (repository.pullRequest)
```

The first three are refused with exit code 2, a branch or a repository that is not there with exit code 3, and an error of `gh` with exit code 1.
