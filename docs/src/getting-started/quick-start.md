# Quick start

A first session with `octok`: one project with two repositories, a workspace on a new branch, a commit pushed from it, a sync and a clean removal.

Every example on this page is run against real git repositories, so the output you see is what `octok` prints.
The repositories are `shop-frontend` and `shop-backend`, two repositories of a company called `acme`.
To follow along, use the URLs of two repositories of yours that you can clone and push to, and the branch your features start from in place of `staging`.
If `octok` is not installed yet, start with [Installation](installation.md).

## Create a project

A [project](../concepts.md#project) is a named group of repositories.
It has a [base branch](../concepts.md#base-branch): the branch every new feature starts from.
The base branch defaults to `staging`.

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
```

`octok` wrote `~/.octotask/projects/shop.yaml` and made `shop` the default project, so the next commands need no `-p shop`.
Everything `octok` keeps is under `~/.octotask`, its [data folder](installation.md#the-data-folder).

At a terminal, `octok project init` also asks for the settings that no flag gives: the base branch and the [location](../concepts.md#location), the machine where the repositories live.
Each question shows its default between brackets, such as `Base branch [staging]:`, and Enter keeps it.
The examples on this page run without a terminal, so no question appears in them.
See [Projects](../commands/project.md) for the other project commands.

## Add repositories

Each [repository](../concepts.md#repository) is registered by its URL.
`--alias` gives it a short name that you can type instead of its key.

```console
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo ls
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git
```

Each repository was cloned once into the [cache](../concepts.md#cache), at `~/.octotask/cache/shop/<key>.git`.
The key of a repository, here `shop-frontend`, is taken from the URL, and it is the name of its folder in every workspace.
At a terminal, `octok repo add` asks for the location and, once the repository is cloned, for its base branch: Enter makes the repository follow the project.
See [Repositories](../commands/repo.md) for `--name`, `--base`, `--location` and those questions.

## Create a workspace

`octok branch` creates the branch `feat/cart` in every repository, from the up-to-date base branch of each one, and checks it out in a new [workspace](../concepts.md#workspace).
It prints the path of the workspace, so `cd "$(...)"` takes you there.

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
```

The workspace is the folder `~/.octotask/workspaces/shop/feat-cart`: its name is the branch with `/` replaced by `-`.
See [Workspaces](../commands/workspaces.md) for the details, and [Pull request review](../commands/pr-review.md) for a workspace that starts from existing pull requests.

## Look around

The workspace has one folder per repository.
Each one is a normal git working tree on the branch `feat/cart`.

```console
$ pwd
/Users/you/.octotask/workspaces/shop/feat-cart
$ ls
shop-backend
shop-frontend
$ git -C shop-backend branch --show-current
feat/cart
$ git -C shop-frontend log --oneline
1ec5189 Add cart page
2736d78 Add product list
17d7cc6 Initial commit
```

The frontend starts from the commit of its base branch, `staging`, which is ahead of `main`.
The folders are git worktrees of the caches, so no repository was cloned again to create them.

## See the state of the project

```console
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/cart  de08597  Fix order totals (5 hours ago)  [not on the remote]
```

`octok status` shows every repository with the last commit of its base branch, and every workspace with the branch and the last commit of each checkout.
`[not on the remote]` says that the branch exists only on your machine.
It does not contact the git remotes, so it answers at once.
See [Status](../commands/status.md).

## Make a commit and push

Work in a checkout as in any git repository.
A plain `git push` publishes the new branch and sets its upstream, with no `-u origin feat/cart`.

```console
$ cd shop-backend
$ echo 'def cart(): pass' > cart.py
$ git add cart.py
$ git commit -q -m 'Add cart endpoint'
$ git push
To git@github.com:acme/shop-backend.git
 * [new branch]      feat/cart -> feat/cart
branch 'feat/cart' set up to track 'origin/feat/cart'.
$ cd ..
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/cart  d32a65a  Add cart endpoint (10 minutes ago)
```

The branch is now on the remote of `shop-backend`, and the status no longer marks it.
`shop-frontend` is still local only: each repository is published on its own, when it has something to publish.

## Sync

`octok pull` fetches every repository and fast-forwards each checkout of the workspace to its remote branch.
Without an argument it uses the workspace of the current folder.

```console
$ octok pull
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    up to date
```

It never merges or rebases: a checkout that cannot fast-forward is skipped and reported.
`octok fetch` only updates the caches.
See [Syncing](../commands/sync.md).

## Remove the workspace

When the feature is done, `octok rm` deletes the workspace: its checkouts, its folder and its local branches.

```console
$ cd
$ octok rm feat/cart
removed workspace `feat-cart` from shop
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)

workspaces: none (create one with `octok branch <name>`)
```

`octok rm` refuses to delete a workspace that holds uncommitted changes, unpushed commits or files that do not belong to a checkout, and lists what it found.
The frontend branch had no commits, so there was nothing to lose here.
The caches stay, so the next workspace is again created in an instant.

## Where to go next

- [Concepts](../concepts.md) defines the words used here.
- [A feature across several repositories](../guides/multi-repo-feature.md) is this session at a realistic size: teammates who push, a base branch that moves, a repository added later and the cleanup after a squash merge.
- [Running](../commands/up.md) starts every repository of a workspace with one command, and [Dev changes](../commands/dev.md) keeps your local settings, such as a `.env`, out of every commit.
- [Reviewing a pull request](../guides/review-pull-request.md) builds a workspace from pull requests.
- [Command reference](../reference/commands.md) lists every command and option.
