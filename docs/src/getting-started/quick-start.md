# Quick start

A first session with `octok`: one project with two repositories, a workspace on a new branch, a commit pushed from it, a sync and a clean removal.

The examples use two repositories of a company called `acme`, `shop-frontend` and `shop-backend`.
To follow along, use two repositories you can clone and push to, and the branch your features start from in place of `staging`.
If `octok` is not installed yet, start with [Installation](installation.md).

## Create a project

A [project](../concepts.md#project) is a named group of repositories, with a [base branch](../concepts.md#base-branch) every new feature starts from, `staging` by default:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
```

`shop` is now the default project, so the next commands need no `-p shop`.
At a terminal, `octok project init` also asks for the base branch and the [location](../concepts.md#location), the machine where the repositories live: Enter keeps the default between brackets.
See [Projects](../commands/project.md).

## Add repositories

Register each [repository](../concepts.md#repository) by its URL, with `--alias` for a short name:

```console
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo ls
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git
```

Each repository is cloned once into the [cache](../concepts.md#cache).
Its key, here `shop-frontend`, comes from the URL and names its folder in every workspace.
At a terminal, `octok repo add` also asks for the location and the base branch: Enter makes the repository follow the project.
See [Repositories](../commands/repo.md).

## Create a workspace

`octok branch` creates `feat/cart` in every repository, from its up-to-date base branch, and checks it out in a new [workspace](../concepts.md#workspace).
It prints the path, so `cd "$(...)"` enters it:

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
```

The workspace is named after the branch, with `/` replaced by `-`.
See [Workspaces](../commands/workspaces.md).

## Look around

The workspace has one folder per repository, each a normal git working tree on `feat/cart`:

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

## See the state of the project

`octok status` shows the last commit of each base branch and of each checkout, without contacting the remotes:

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

`[not on the remote]` marks a branch that exists only on your machine.
See [Status](../commands/status.md).

## Make a commit and push

Work in a checkout as in any git repository.
A plain `git push` publishes the new branch and sets its upstream:

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

Each repository is published on its own, so `shop-frontend` is still local only.

## Sync

`octok pull` fetches every repository and fast-forwards each checkout of the current workspace:

```console
$ octok pull
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    up to date
```

It never merges or rebases: a checkout that cannot fast-forward is skipped and reported.
`octok fetch` only updates the caches.
See [Syncing](../commands/sync.md).

## Remove the workspace

When the feature is done, `octok rm` deletes the checkouts, the folder and the local branches of the workspace:

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

`octok rm` refuses a workspace with uncommitted changes, unpushed commits or files outside a checkout, and lists them.
The caches stay, so the next workspace is again ready in an instant.

## Where to go next

- [Concepts](../concepts.md) defines the words used here.
- [A feature across several repositories](../guides/multi-repo-feature.md) is this session at a realistic size.
- [Running](../commands/up.md) starts every repository of a workspace, and [Dev changes](../commands/dev.md) keeps local settings, such as a `.env`, out of every commit.
- [Reviewing a pull request](../guides/review-pull-request.md) builds a workspace from pull requests.
- [Command reference](../reference/commands.md) lists every command and option.
