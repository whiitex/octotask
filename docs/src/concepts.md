# Concepts

The words `octok` uses (project, repository, base branch, cache, workspace, location, dev changes), how they relate and where each one lives on disk.

For a project `shop` with two repositories and one workspace, the data folder looks like this:

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

Every workspace shares the cache, so creating one never clones a repository again.

## Project

A project is a named group of repositories that make up one product, such as a frontend, a backend and an agent.
Its file `~/.octotask/projects/<project>.yaml` holds the repositories, the default [base branch](#base-branch) and [location](#location), and the settings of its workspaces.

- **Create it** with `octok project init <name>`.
- **The default project** is the one commands act on without `-p <project>`, as in `octok -p shop status`, and `octok project use <name>` changes it.
- **A name** is up to 64 letters, digits, `.`, `_` and `-`, starting with a letter or a digit.
- **Several projects** can share a repository, each with its own cache, workspaces and file.

See [Projects](commands/project.md) and [Configuration files](reference/configuration.md).

## Repository

A repository is a git repository registered in a project, with:

- **a URL**: the scp-like `git@github.com:acme/shop-backend.git`, an `https://`, `http://`, `ssh://`, `git://` or `file://` URL, or an absolute path, with no spaces.
- **a key**, its identity in the project and the name of its folder in every workspace.
  It defaults to the last part of the URL without `.git`, and it never changes.
  `octok repo add --name <key>` chooses another one, as needed when two repositories have the same name.
- **an optional alias**, a shorter name accepted wherever a key is, as in `octok repo set backend --base main`.
  Keys and aliases share one namespace, so a name always points to one repository.
- **optionally**, its own base branch and location, and the commands `octok up` runs.

Keys and aliases follow the rules of project names, and commands print a repository as `key (alias)`.
See [Repositories](commands/repo.md).

## Base branch

The base branch is the branch a new feature starts from, usually `staging`, `develop` or `main`.

- **The project** has one, `staging` unless `octok project init --base` says otherwise.
- **A repository** can have its own; without one it follows the project, also when the project base changes later.
- **`octok branch <branch>`** fetches every repository and creates the branch from the base of each one as it is on its remote, so two repositories can start from different branches.
- **A base branch not on the remote** needs a confirmation, and is refused when `octok` cannot ask.

Here the frontend follows the project and the backend has its own base:

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

## Cache

The cache of a repository is a bare clone at `~/.octotask/cache/<project>/<key>.git`, made when the repository is added.

- **It is updated** by `octok fetch`, `octok pull` and every command that creates a workspace.
- **It is for `octok` only:** do not work in it.
- **It is deleted** with its repository (`octok repo rm`) or its project (`octok project rm`), and `octok fetch` clones a missing one again.

## Workspace

A workspace is the folder of one feature: a checkout of every repository, all on the same branch.

- **Its name** is the branch with `/` replaced by `-`: `feat/login` lives in `feat-login`.
  `feat/login` and `feat-login` give the same name, so only one of them can have a workspace.
- **A checkout** is a git worktree of the cache, in the folder named after the key, where `git` works as usual.
  A branch can be checked out in one place at a time, so every workspace has its own branches.
- **Its path** is `~/.octotask/workspaces/<project>/<workspace>`, printed by the commands that create it, so `cd "$(octok branch feat/login)"` enters it.
- **Create it** with `octok branch` (a new branch), `octok checkout` (a pushed branch) or `octok pr review` (pull requests and branches), and delete it with `octok rm`.

See [Workspaces](commands/workspaces.md) and [Pull request review](commands/pr-review.md).

## Location

A location is the machine where a repository's cache and checkouts live: `local`, this machine, by default.
For another machine, such as a VM, `octok` runs `git` there over `ssh`.

- **Its form** is `local`, `<host>` or `<host>:<path>`, where `<host>` is anything `ssh` accepts, such as a `Host` of `~/.ssh/config` or `user@host`.
  With only the host the folder is `.octotask` in the home of the user there, and a relative `<path>` is under that home.
- **The first level set wins:**
  1. the workspace, with `octok branch <branch> --location <location>`, for every repository of it;
  2. the repository, with `--location` on `octok repo add` or `octok repo set`;
  3. the project, with `--location` on `octok project init` or `octok project set`.
- **A workspace can span machines:** one folder on each, and `octok status`, `octok pull` and `octok rm` reach all of them.

See [Locations and SSH](commands/locations.md) and [Repositories on another machine](guides/remote-machine.md).

## Dev changes

Dev changes are the local-only edits a repository needs to run on your machine, such as a port, a disabled line or a `.env` file.
You save them once with `octok dev save`, and `octok` puts them into every new checkout.
They do not show in `git status` or `git diff`, and no commit contains them.
They are kept in `~/.octotask/dev/<project>/<key>` on the machine where the repository lives.
See [Dev changes](commands/dev.md).

## Related

- [Configuration files](reference/configuration.md) shows the files that store all of this.
- [Quick start](getting-started/quick-start.md) uses each concept in a first session.
