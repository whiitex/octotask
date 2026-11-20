# Configuration files

Every file and folder that `octok` writes: the layout of the data folder on this machine and on another one, every field of the project file and of `config.yaml`, and what happens when you edit one by hand.

`octok` keeps all its state under one folder, the data folder.
It is `~/.octotask` unless the environment variable `OCTOTASK_HOME` names another one.
Nothing is written outside it, except the control sockets of [ssh connections](../commands/locations.md) in `~/.ssh`.
The vocabulary used here (project, repository, base branch, cache, workspace, location) is defined in [Concepts](../concepts.md).

## The data folder

| Path | Content |
|---|---|
| `config.yaml` | The global config: the default project. |
| `registry.lock` | The lock that every change to the files above and below takes. |
| `projects/<project>.yaml` | The project file: its repositories and the settings of its workspaces. |
| `cache/<project>/<repo>.git` | The cache: the bare clone of a repository. |
| `cache/<project>/<repo>.lock` | The lock that every operation on that cache takes. |
| `workspaces/<project>/<workspace>/<repo>` | The checkouts of a workspace: one git worktree of the cache for each repository. |
| `dev/<project>/<repo>/` | The dev changes saved for a repository: `dev/changed`, `dev/new`, `base` and `filter.sh`. |

`<repo>` is always the key of the repository, never its alias.
A folder is created when it is first needed: a new data folder is empty, and the first `octok project init` writes `registry.lock`, the project file and `config.yaml`.

The examples on this page share one project, `shop`.
Its repositories live on another machine by default, and `shop-backend` lives on this one.
A file is saved as a dev change in `shop-backend`, and two workspaces exist.
This is the data folder on this machine, down to three levels:

```console
$ octok project init shop --location api-dev-1
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
its repos live at api-dev-1:/home/ubuntu/.octotask
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
cloned at api-dev-1:/home/ubuntu/.octotask
$ octok repo add git@github.com:acme/shop-backend.git --alias backend --base main --location local
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo set frontend --setup 'npm ci' --run 'npm run dev'
updated `shop-frontend (frontend)` in shop
$ octok pr review --pr backend=12 --branch frontend=feat/login
shop-backend (backend): pull request #12 is the branch `feat/login`
created workspace `pr-backend-12` in shop
  branch from the remote: shop-frontend, shop-backend
  shop-frontend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/pr-backend-12/shop-frontend
/Users/you/.octotask/workspaces/shop/pr-backend-12
$ cd "$(octok branch feat/cart)/shop-backend"
created workspace `feat-cart` in shop
  shop-frontend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-frontend
$ echo 'TOKEN=1' > .env
$ octok dev save --file .env
shop-frontend (frontend)
  no dev changes
shop-backend (backend)
  new      .env
$ cd
$ octok repo set backend --run 'PORT=9005 uv run main.py' --branch feat/cart
updated `shop-backend` in workspace `feat-cart` of shop
$ find .octotask -maxdepth 3 | sort
.octotask
.octotask/cache
.octotask/cache/shop
.octotask/cache/shop/shop-backend.git
.octotask/cache/shop/shop-backend.lock
.octotask/cache/shop/shop-frontend.lock
.octotask/config.yaml
.octotask/dev
.octotask/dev/shop
.octotask/dev/shop/shop-backend
.octotask/projects
.octotask/projects/shop.yaml
.octotask/registry.lock
.octotask/workspaces
.octotask/workspaces/shop
.octotask/workspaces/shop/feat-cart
.octotask/workspaces/shop/pr-backend-12
```

The dev changes of a repository are a folder of their own, kept on the machine of the repository:

```console
$ find .octotask/dev | sort
.octotask/dev
.octotask/dev/shop
.octotask/dev/shop/shop-backend
.octotask/dev/shop/shop-backend/base
.octotask/dev/shop/shop-backend/dev
.octotask/dev/shop/shop-backend/dev/changed
.octotask/dev/shop/shop-backend/dev/new
.octotask/dev/shop/shop-backend/dev/new/.env
.octotask/dev/shop/shop-backend/filter.sh
```

The files of git inside `cache/` and of the checkouts are the ones git writes: `octok` runs your own `git`, so every checkout is a normal working tree.
Its `.git` is a file that points back to the cache.

### On another machine

A repository whose [location](../concepts.md#location) is another machine keeps its cache, its checkouts and its dev changes in the data folder of that machine, and nothing else.
The location says which folder: `api-dev-1:/home/ubuntu/.octotask` is the folder `/home/ubuntu/.octotask` on the host `api-dev-1`.
The project file, `config.yaml` and `registry.lock` stay on this machine only, so the other machine has no `projects/` folder and no lock file.

```console
$ ssh api-dev-1 'find .octotask -maxdepth 4 | sort'
.octotask
.octotask/cache
.octotask/cache/shop
.octotask/cache/shop/shop-frontend.git
.octotask/cache/shop/shop-frontend.git/FETCH_HEAD
.octotask/cache/shop/shop-frontend.git/HEAD
.octotask/cache/shop/shop-frontend.git/config
.octotask/cache/shop/shop-frontend.git/description
.octotask/cache/shop/shop-frontend.git/hooks
.octotask/cache/shop/shop-frontend.git/info
.octotask/cache/shop/shop-frontend.git/objects
.octotask/cache/shop/shop-frontend.git/refs
.octotask/cache/shop/shop-frontend.git/worktrees
.octotask/dev
.octotask/dev/shop
.octotask/workspaces
.octotask/workspaces/shop
.octotask/workspaces/shop/feat-cart
.octotask/workspaces/shop/feat-cart/shop-frontend
.octotask/workspaces/shop/pr-backend-12
.octotask/workspaces/shop/pr-backend-12/shop-frontend
```

The locks of a cache are taken on this machine, so `cache/shop/shop-frontend.lock` is in the local data folder even though the cache is not.
Everything `octok` reads from the other machine it asks over `ssh`: see [Locations and SSH](../commands/locations.md).

## The project file

`projects/<project>.yaml` holds one project.
`octok` writes it whenever a command changes the project, and reads it at the start of every command.
This is the file of `shop`: a location, two repositories that set their own values, a workspace with its own run command and a workspace that reviews a pull request:

```console
$ cat .octotask/projects/shop.yaml
version: 1
name: shop
base: staging
location: api-dev-1:/home/ubuntu/.octotask
repos:
  shop-frontend:
    alias: frontend
    url: git@github.com:acme/shop-frontend.git
    setup: npm ci
    run: npm run dev
  shop-backend:
    alias: backend
    url: git@github.com:acme/shop-backend.git
    base: main
    location: local
workspaces:
  pr-backend-12:
    branch: pr-backend-12
    repos:
      shop-frontend:
        branch: feat/login
      shop-backend:
        branch: feat/login
  feat-cart:
    repos:
      shop-backend:
        run: PORT=9005 uv run main.py
```

Only `version`, `name`, `base` and `repos` are always written, `repos` as `repos: {}` while the project has none.
Any other field with no value is left out of the file, not written as `null`.
The keys of `repos` and `workspaces` keep the order in which they were added.

### Project

| Field | Required | Value |
|---|---|---|
| `version` | yes | The version of the format: `1`. See [The version field](#the-version-field). |
| `name` | yes | The name of the project. It must be the file name without `.yaml`. |
| `base` | yes | The [base branch](../concepts.md#base-branch) of every repository that does not set its own. |
| `location` | no | Where the repositories live unless they say otherwise. Left out means this machine. |
| `repos` | no | The registered repositories, by key. Left out when you edit the file means none; `octok` always writes it. |
| `workspaces` | no | The settings of the workspaces that have any, by workspace name. |

### Repositories

Each entry of `repos` is a repository, under its key.

| Field | Required | Value |
|---|---|---|
| `alias` | no | A short name accepted wherever the key is. |
| `url` | yes | The git URL the cache is cloned and fetched from. |
| `base` | no | The base branch of this repository, instead of the one of the project. |
| `location` | no | Where this repository lives, instead of the location of the project. |
| `setup` | no | The shell command `octok up` runs to prepare a checkout before its first run, such as `npm ci`. |
| `run` | no | The shell command `octok up` runs to start the repository, in its checkout. |

### Workspaces

An entry of `workspaces` exists only while its workspace has a setting of its own, and goes away with the last one.
It is named like the workspace folder: the branch with `/` replaced by `-`.

| Field | Required | Value |
|---|---|---|
| `location` | no | Where every repository of the workspace lives, whatever the repositories and the project say. Set by `--location` on `octok branch`, `octok checkout` or `octok pr review`. |
| `branch` | no | The branch of the workspace. Saved by `octok pr review`, because some repositories of that workspace are on another branch. |
| `repos` | no | The settings of single repositories in this workspace, by key. |

Each entry of a workspace `repos` has:

| Field | Required | Value |
|---|---|---|
| `run` | no | The command that starts the repository in this workspace, instead of the `run` of the repository. Set by `octok repo set --run --branch`. |
| `branch` | no | The branch the repository has in this workspace, instead of the branch of the workspace. Saved by `octok pr review`. |

The workspace entry of a review stays until `octok rm`, which also forgets the other settings of that workspace: see [Pull request review](../commands/pr-review.md).

### Allowed values

| Value | Rule |
|---|---|
| Project name, repository key, alias | 1 to 64 characters: letters, digits, `.`, `_` and `-`, starting with a letter or a digit. |
| Key and alias | They share one namespace: no alias equals a key or the alias of another repository. |
| Branch | A valid git branch name: not empty, not starting with `-`, no whitespace or control characters, none of `~ ^ : ? * [ \`, no `..`, no `@{`, no part starting with `.` or ending with `.lock`, not ending with `.`, not `@` or `HEAD`. |
| `url` | `https://`, `http://`, `ssh://`, `git://` or `file://` followed by a path, an absolute path, or the `[user@]host:path` form. No whitespace. |
| Location | `local`, or `<host>:<absolute path>`. The host is letters, digits, `.`, `_`, `-` and `@`, and does not start with `-`. The path has no control characters. |
| `setup`, `run` | Any text that is not empty or only blanks. It is handed to `sh` as it is. |
| Workspace name | Not empty and without `/`. |
| Keys under a workspace `repos` | They must be repositories of the project. |

The location in the file is always the absolute one, such as `api-dev-1:/home/ubuntu/.octotask`.
The commands accept `<host>` alone or a relative path and write the absolute path: see [Locations and SSH](../commands/locations.md).
The key of a repository comes from its URL, as the last part without `.git`, unless `octok repo add --name` gives another.

The `run` and `setup` commands are plain text in this file, and so is everything else in it.
Do not put a secret in a command: keep it in a `.env` file saved as a [dev change](../commands/dev.md), which never leaves the machine of the repository.

## The global config

`config.yaml` holds the settings that belong to no project.

| Field | Required | Value |
|---|---|---|
| `version` | yes | The version of the format: `1`. |
| `default_project` | no | The project that commands act on when `-p` is not given. Left out when there is none. |

```console
$ cat .octotask/config.yaml
version: 1
default_project: shop
```

`octok project init` and `octok project use` set `default_project`, and `octok project rm` clears it when it names the project that is removed.
A data folder without `config.yaml` is valid: it has no default project.

## The version field

Both kinds of file start with `version: 1`.
`octok` reads the version first, and refuses a file with another one before looking at anything else, so a file written by a newer `octok` fails with a clear message and not with a list of unknown fields.
Nothing is migrated: there is only one version so far.

## Editing by hand

You can edit a project file or `config.yaml` with an editor.
`octok` reads the file again at the next command, so the change applies at once.
Three things to know:

- **The whole file is checked on every read.**
  A value that breaks a rule above makes every command that uses the project fail, and nothing is changed.
  The error names the file when the file cannot be read, as in the first four examples below, and only the value when it is read but breaks another rule, as in the fifth.
- **`octok` rewrites the whole file when it changes something.**
  Comments, blank lines and the formatting of your edit are lost, and the fields come back in the order of the tables above.
- **A write replaces the file as it was read.**
  An edit saved while an `octok` command is running can be lost if that command writes after it.

These are the errors you get from a file that is not valid.
Each `printf` replaces the project file or `config.yaml` with a small invalid one, and the next command shows the error:

```console
$ cp .octotask/projects/shop.yaml shop.yaml.bak
$ printf 'version: 2\nname: shop\nbase: staging\n' > .octotask/projects/shop.yaml
$ octok status
error: /Users/you/.octotask/projects/shop.yaml: unsupported config version 2 (expected 1)
$ printf 'version: 1\nname: other\nbase: staging\n' > .octotask/projects/shop.yaml
$ octok status
error: /Users/you/.octotask/projects/shop.yaml: file declares project `other` but is named after `shop`
$ printf 'version: 1\nname: shop\nbase: staging\nrepo: {}\n' > .octotask/projects/shop.yaml
$ octok status
error: /Users/you/.octotask/projects/shop.yaml: error: line 4 column 1: unknown field `repo`, expected one of version, name, base, location, repos, workspaces
 --> <input>:4:1
  |
2 | name: shop
3 | base: staging
4 | repo: {}
  | ^ unknown field `repo`, expected one of version, name, base, location, repos, workspaces
$ printf 'version: 1\nname: shop\nbase: staging\nlocation: api-dev-1\n' > .octotask/projects/shop.yaml
$ octok status
error: /Users/you/.octotask/projects/shop.yaml: error: line 4 column 11: invalid location `api-dev-1`: expected `local` or `<host>:<absolute path>`
 --> <input>:4:11
  |
2 | name: shop
3 | base: staging
4 | location: api-dev-1
  |           ^ invalid location `api-dev-1`: expected `local` or `<host>:<absolute path>`
$ printf 'version: 1\nname: shop\nbase: staging\nworkspaces:\n  feat-x:\n    repos:\n      ghost:\n        run: x\n' > .octotask/projects/shop.yaml
$ octok status
error: repo `ghost` is not registered
$ printf 'oops: 1\n' > .octotask/config.yaml
$ octok project ls
error: /Users/you/.octotask/config.yaml: error: line 1 column 1: missing field `version`
 --> <input>:1:1
  |
1 | oops: 1
  | ^ missing field `version`
$ printf 'version: 3\n' > .octotask/config.yaml
$ octok project ls
error: /Users/you/.octotask/config.yaml: unsupported config version 3 (expected 1)
```

The first error is a version the file declares that `octok` does not know.
The second is a file whose `name` differs from its file name, which happens when you copy a project file to another name.
The third is a field that does not exist: the fields are checked strictly, so a typo is an error and not a setting that is silently ignored.
The fourth is a value that breaks a rule, here a location without a path.
The fifth is a workspace setting for a repository that is not in the project.
The last two are the same errors in `config.yaml`.

To start again from the files `octok` wrote, put the copy back.
The next commands also show that `octok` writes a file again from scratch: the comment added at the end is gone after `octok repo set`.

```console
$ cp shop.yaml.bak .octotask/projects/shop.yaml
$ rm .octotask/config.yaml
$ echo '# a note' >> .octotask/projects/shop.yaml
$ octok -p shop repo set backend --alias api
updated `shop-backend (api)` in shop
$ tail -3 .octotask/projects/shop.yaml
    repos:
      shop-backend:
        run: PORT=9005 uv run main.py
$ octok project ls
  shop
$ octok project use shop
default project: shop
$ octok project ls
* shop
```

`octok project ls` lists the files in `projects/` whose name is `<project>.yaml` with a valid project name, without reading them, so a broken file still shows in the list.
A file in `projects/` that has any other name is ignored.

## Lock files

`registry.lock` and `cache/<project>/<repo>.lock` are empty files that serve as locks.
Every command that changes a project file or `config.yaml` holds an exclusive lock on `registry.lock` until it ends, and every operation on a cache holds the one of that cache, so two `octok` commands that run at the same time, such as two terminals or an agent next to you, never write the same file at once.
Reading a file takes no lock, because every write is made to a temporary file in the same folder and then renamed over the real one: a reader sees the old file or the new one, never half of it.

The operating system releases a lock when the process ends, even when it is killed, so a lock file left behind is not a stuck lock: leave it where it is.
`octok` never deletes `registry.lock`, and deletes a cache lock only together with its cache.
A command that finds the lock taken waits for it.
A temporary file named `.<file>.tmp-<process id>` can stay in a folder only if a process was killed in the middle of a write: it is safe to delete.
A folder `cache/<project>/<repo>.tmp-<process id>` is a clone that was interrupted, for example by `Ctrl-C`: the next clone of that repository deletes it.

```console
$ find .octotask -name '*.lock' | sort
.octotask/cache/shop/shop-backend.lock
.octotask/cache/shop/shop-frontend.lock
.octotask/registry.lock
```

## Related

- [Concepts](../concepts.md) defines the words used here.
- [Projects](../commands/project.md), [Repositories](../commands/repo.md) and [Workspaces](../commands/workspaces.md) describe the commands that write these files.
- [Locations and SSH](../commands/locations.md) explains the data folder on another machine.
- [Dev changes](../commands/dev.md) explains what is in `dev/`.
- [Troubleshooting](../troubleshooting.md) covers the errors of an invalid file.
