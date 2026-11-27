# Introduction

The CLI `octok` runs the same git operation on every repository of a project, so a feature that touches a frontend, a backend and an agent is one branch, one folder and one command.

## How it works

- You group the repositories of a product into a [project](concepts.md#project).
- `octok` clones each repository once into a [cache](concepts.md#cache) and keeps it up to date.
- For each feature it creates a [workspace](concepts.md#workspace): one folder with a checkout of every repository, all on the same branch.
- Each checkout is a `git worktree` of the cache, so a workspace is ready in an instant, and several can be open side by side without stashing or switching branches.

`octok` calls your own `git`, so your credentials, SSH configuration and hooks apply.
Every checkout is a normal working tree: you commit, push and open pull requests with the tools you already use.

## The core loop

Create a project, add its repositories, then create a workspace for a feature and look at the state of everything:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git

$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ ls
shop-backend
shop-frontend

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

`octok branch feat/cart` fetched both repositories, created `feat/cart` in each one from its up-to-date base branch, and checked it out in a new workspace.
It prints the path of the workspace, so `cd "$(...)"` enters it.
From there, `git` is all you need in each repository, and `octok rm feat/cart` deletes the workspace when the feature is done.

## What else it does

- **[A teammate's branch](commands/workspaces.md):** `octok checkout feat/login` builds the workspace of an existing branch, and the repositories without it start from their base.
- **[Pull requests side by side](commands/pr-review.md):** `octok pr review --pr backend=12` puts each repository on the pull request or branch you choose, and the others on their base.
- **[Repositories on other machines](commands/locations.md):** a repository can live on a machine reached over `ssh`, such as a VM.
- **[One command to start everything](commands/up.md):** `octok up` starts every repository of a workspace, shows their output together and stops them all at `Ctrl-C`.
- **[Dev changes that follow you](commands/dev.md):** local-only edits and files such as `.env` are saved once, put into every new workspace and kept out of every commit.
- **[Ready for coding agents](agents/overview.md):** every command prints JSON with `--json` and reports failures with its exit code, and a skill teaches an agent to drive `octok`.

## Where to read next

New to `octok`: read [Installation](getting-started/installation.md), the [Quick start](getting-started/quick-start.md) and [Concepts](concepts.md).
Then:

| To | Read |
|---|---|
| Manage projects and repositories | [Projects](commands/project.md), [Repositories](commands/repo.md) |
| Update the checkouts | [Syncing](commands/sync.md) |
| See the state of a project | [Status](commands/status.md) |
| Open a workspace in your editor | [Opening in an editor](commands/open.md) |
| Follow a real feature, review or remote setup | [A feature across several repositories](guides/multi-repo-feature.md), [Reviewing a pull request](guides/review-pull-request.md), [Repositories on another machine](guides/remote-machine.md) |
| Parse the output of `octok` | [JSON output](agents/json-output.md) |
| Look up a command or an option | [Command reference](reference/commands.md) |
| Know what files `octok` writes | [Configuration files](reference/configuration.md) |
| Fix an error | [Troubleshooting](troubleshooting.md) |
| Read these pages in a terminal | [Built-in documentation](commands/docs.md) |
| Build and change `octok` itself | [Development](development.md) |
