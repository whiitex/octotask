# Concepts

The words `octok` uses (project, repository, base branch, cache, workspace, location, dev changes), how they relate and where each one lives on disk.

`octok` runs the same git operation on every repository of a project.
To do that it keeps a few things apart: the list of repositories, one clone of each, and one set of checkouts for each feature.
The sections below define each of them.
The other pages link here for the definitions.

This is how they fit together on disk, for a project `shop` with two repositories and one workspace:

```text
~/.octotask/
  projects/shop.yaml                   the project: its repositories and their settings
  cache/shop/shop-frontend.git         the cache of a repository: one bare clone
  cache/shop/shop-backend.git
  workspaces/shop/feat-login/          a workspace: named after its branch
    shop-frontend/                     a checkout: a git worktree of the cache
    shop-backend/
  dev/shop/shop-backend/               the dev changes saved for a repository
```

The cache is shared by every workspace.
A checkout is a git worktree of the cache, so creating a workspace never clones a repository again.

## Project

A project is a named group of repositories that make up one product, such as a frontend, a backend and an agent.
It is stored in one file, `~/.octotask/projects/<project>.yaml`, and holds:

- the repositories, each with its settings;
- the default [base branch](#base-branch) of the repositories;
- optionally, the default [location](#location) of the repositories;
- the settings of the workspaces that have any.

You create it with `octok project init <name>`.
One project is the default: commands act on it unless you pass `-p <project>`, for example `octok -p shop status`.
`octok project use <name>` changes the default.
Several projects can share the same repository: each one has its own cache, its own workspaces and its own file.

A project name is a name for a file: up to 64 letters, digits, `.`, `_` and `-`, starting with a letter or a digit.
See [Projects](commands/project.md) for the commands and [Configuration files](reference/configuration.md) for the file.

## Repository

A repository is a git repository registered in a project.
It has:

- a **URL**, which `octok` clones and fetches from.
  It can be the scp-like form `git@github.com:acme/shop-backend.git`, an `https://`, `http://`, `ssh://`, `git://` or `file://` URL, or an absolute path, with no spaces.
- a **key**, its identity in the project and the name of its folder in every workspace.
  It defaults to the last part of the URL without `.git`: `git@github.com:acme/shop-backend.git` gives `shop-backend`.
  `octok repo add --name <key>` chooses another one, which is needed when two repositories have the same name.
- an optional **alias**, a shorter name accepted wherever a key is: `octok repo set backend --base main` works the same as `octok repo set shop-backend --base main`.
  Keys and aliases share one namespace, so a name always points to one repository.
- optionally its own base branch, its own location, and the commands `octok up` runs to set it up and start it.

Keys and aliases follow the same rules as project names.
The key never changes once the repository is registered, because it names folders.
Commands print a repository as `key (alias)`, or only `key` when there is no alias.
See [Repositories](commands/repo.md).

## Base branch

The base branch is the branch a new feature starts from, usually `staging`, `develop` or `main`.
It is set at two levels:

- **The project** has a base branch, `staging` unless `octok project init --base` says otherwise.
- **A repository** can override it with its own, for the repository whose long-lived branch has another name.

A repository without its own base follows the project, including when the project base changes later.
`octok branch <branch>` fetches every repository and creates the new branch from the base branch of each one, as it is on its remote.
Two repositories of a workspace can therefore start from different branches.
`octok status` shows the base branch of each repository and its last commit.

When a base branch is not on the remote, `octok` asks for confirmation before using it, and refuses when it cannot ask.

```console
$ octok project init shop --base staging
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend --base main
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo ls
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git
shop-backend (backend)    main     git@github.com:acme/shop-backend.git
```

Here the frontend follows the project and the backend has its own base.

## Cache

The cache of a repository is a bare clone of it, kept at `~/.octotask/cache/<project>/<key>.git`.
It is made once, when the repository is added, and updated by `octok fetch`, `octok pull` and every command that creates a workspace.
All the workspaces of the project are made from it, which is why creating one is fast and does not download the repository again.

A cache is for `octok` only: do not work in it.
It holds the objects and the branches of the remote, plus the local branches of the workspaces.
It is deleted with its repository (`octok repo rm`) and with its project (`octok project rm`).
`octok fetch` clones again a cache that is missing.

## Workspace

A workspace is the folder that holds the work on one feature: one checkout of every repository of the project, all on the same branch.

- **Its name comes from the branch**, with `/` replaced by `-`: the branch `feat/login` lives in the workspace `feat-login`.
  `feat/login` and `feat-login` give the same name, so only one of them can have a workspace.
- **A checkout** is the working tree of one repository inside the workspace: a git worktree of its cache, in the folder named after the key of the repository.
  It is a normal working tree: `git status`, `git commit` and `git push` work in it.
  A branch can be checked out in one place at a time, which is why every workspace has its own branches.
- **Its path** is `~/.octotask/workspaces/<project>/<workspace>`.
  `octok branch`, `octok checkout` and `octok pr review` print it, so that `cd "$(octok branch feat/login)"` enters it.

You create a workspace with `octok branch` (a new branch), `octok checkout` (a branch someone already pushed) or `octok pr review` (a choice of pull requests and branches), and delete it with `octok rm`.
Workspaces are isolated: several features can be open side by side without stashing or switching branches.
See [Workspaces](commands/workspaces.md) and [Pull request review](commands/pr-review.md).

## Location

A location is the machine where a repository's cache and checkouts live.
By default it is this machine, under `~/.octotask`, written `local`.
A repository can live on another machine instead, such as a VM where the backend runs.
`octok` still runs only on your machine: for a repository on another machine it runs `git` there over `ssh`.

A location is `local`, `<host>`, or `<host>:<path>`.
`<host>` is anything `ssh` accepts, such as a `Host` of `~/.ssh/config` or `user@host`.
With only the host, the folder is `.octotask` in the home of the user there, and a relative `<path>` is under that home.
It is set at three levels, and the first one set wins:

1. the workspace, with `octok branch <branch> --location <location>`: every repository of that workspace lives there;
2. the repository, with `--location` on `octok repo add` or `octok repo set`;
3. the project, with `--location` on `octok project init` or `octok project set`.

When none is set, the repository lives on this machine.
A workspace can then have one folder on this machine and one on each other machine, and `octok status`, `octok pull` and `octok rm` reach all of them.
See [Locations and SSH](commands/locations.md) and the guide [Repositories on another machine](guides/remote-machine.md).

## Dev changes

Dev changes are the local-only edits a repository needs to run on your machine and that must never be committed: a port, a database name, a line turned off, a `.env` file.
You save them once from a checkout with `octok dev save`, and `octok` puts them into every checkout it creates afterwards.
git does not show them in `git status` or `git diff`, and no commit contains them.
They are kept per repository, in `~/.octotask/dev/<project>/<key>` on the machine where the repository lives.
See [Dev changes](commands/dev.md).

## Related

- [Configuration files](reference/configuration.md) shows the files that store all of this.
- [Quick start](getting-started/quick-start.md) uses each concept in a first session.
