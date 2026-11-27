# Introduction

The CLI `octok` runs the same git operation on every repository of a project, so a feature that touches a frontend, a backend and an agent is one branch, one folder and one command.

A feature often touches several repositories at once.
Working on it means creating the same branch in each of them, keeping them up to date, remembering where each one stands and starting them together.
`octok` does that for all of them at once.

## How it works

You group the repositories of a product into a [project](concepts.md#project).
`octok` clones each repository once into a [cache](concepts.md#cache), a bare clone that it keeps up to date.
For each feature, it creates a [workspace](concepts.md#workspace): one folder with a checkout of every repository, all on the same branch.
Each checkout is a `git worktree` of the cache, so a workspace is created in an instant, and several workspaces can be open side by side without stashing or switching branches.

Underneath it is plain git.
`octok` calls your own `git`, so your credentials, SSH configuration and hooks apply, and every checkout is a normal working tree: you commit, push and open pull requests with the tools you already use.
`octok` adds the part that git does not have, which is doing it for every repository of the project.

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

`octok branch feat/cart` fetched both repositories, created the branch `feat/cart` in each one from its up-to-date base branch, and checked it out in a new workspace.
The `cd` works because `octok` prints the path of the workspace: a program cannot change the folder of the shell that started it.
From there, `git` is all you need to work in each repository.
`octok status` shows the repositories and every workspace.
`octok rm feat/cart` deletes the workspace when the feature is done.

## What else it does

- **A teammate's branch in one command.** `octok checkout feat/login` builds the workspace of a branch that already exists: each repository that has the branch gets it, the others start from their base.
- **Pull requests side by side.** `octok pr review --pr backend=12` builds a workspace with each repository on the pull request or branch you choose, and the others on their base.
- **Repositories on other machines.** A repository can live on another machine, such as a VM, reached over `ssh`: `octok` runs the same commands there.
- **One command to start everything.** `octok up` starts every repository of a workspace, shows their output together and stops them all at `Ctrl-C`.
- **Dev changes that follow you.** The local-only edits a repository needs to run on your machine, and files such as `.env`, are saved once, put into every new workspace and kept out of every commit.
- **Ready for coding agents.** Every command prints JSON with `--json` and says how it failed with its exit code, and a skill teaches an agent to drive `octok`.

## Where to read next

If you are new, read [Installation](getting-started/installation.md), then the [Quick start](getting-started/quick-start.md) and [Concepts](concepts.md).
Then:

| To | Read |
|---|---|
| Install `octok`, update it or remove it | [Installation](getting-started/installation.md) |
| Follow a first session from a project to a removed workspace | [Quick start](getting-started/quick-start.md) |
| Learn what a project, a base branch, a cache, a workspace and a location are | [Concepts](concepts.md) |
| Create projects and register repositories | [Projects](commands/project.md), [Repositories](commands/repo.md) |
| Create, enter and delete workspaces | [Workspaces](commands/workspaces.md) |
| Review pull requests | [Pull request review](commands/pr-review.md), [Reviewing a pull request](guides/review-pull-request.md) |
| Keep repositories on another machine | [Locations and SSH](commands/locations.md), [Repositories on another machine](guides/remote-machine.md) |
| Update the checkouts | [Syncing](commands/sync.md) |
| Start every repository of a workspace | [Running](commands/up.md) |
| Open the repositories of a workspace in your editor | [Opening in an editor](commands/open.md) |
| Keep local-only changes out of every commit | [Dev changes](commands/dev.md) |
| See the state of a project | [Status](commands/status.md) |
| Work through a feature across several repositories | [A feature across several repositories](guides/multi-repo-feature.md) |
| Use `octok` from a coding agent, or parse its output | [Driving octok from an agent](agents/overview.md), [JSON output](agents/json-output.md) |
| Look up a command or an option | [Command reference](reference/commands.md) |
| Know what files `octok` writes | [Configuration files](reference/configuration.md) |
| Fix an error | [Troubleshooting](troubleshooting.md) |
| Read the documentation of the installed version in a terminal | [Built-in documentation](commands/docs.md) |
| Build and change `octok` itself | [Development](development.md) |
