# Configuration files

Every file and folder that `octok` writes, every field of the project file and of `config.yaml`, and what happens when you edit one by hand.

`octok` keeps all its state in the data folder: `~/.octotask`, or the folder that `OCTOTASK_HOME` names.
Nothing is written outside it, except the control sockets of [ssh connections](../commands/locations.md) in `~/.ssh`.

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

- **`<repo>`** is always the key of the repository, never its alias.
- **A folder is created when first needed:** a new data folder is empty, and the first `octok project init` writes `registry.lock`, the project file and `config.yaml`.
- **Checkouts are normal git working trees**, written by your own `git`.
  Their `.git` is a file that points back to the cache.
- **On another machine:** a repository whose [location](../concepts.md#location) is another machine keeps its cache, its checkouts and its dev changes in the data folder there, and nothing else.
  The location names that folder: `api-dev-1:/home/ubuntu/.octotask` is `/home/ubuntu/.octotask` on the host `api-dev-1`.
  The project file, `config.yaml` and every lock stay on this machine, so `cache/shop/shop-frontend.lock` is local even when its cache is not.

## Example: the data folder

The examples of this page share the project `shop`: its repositories live on `api-dev-1`, except `shop-backend`, which lives here and has a saved dev change.

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

## The project file

`projects/<project>.yaml` holds one project.
This is the file of `shop`, with a workspace that has its own run command and one that reviews a pull request:

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

- **Always written:** `version`, `name`, `base` and `repos`, as `repos: {}` while the project has none.
  Any other field with no value is left out, not written as `null`.
- **Order:** the keys of `repos` and `workspaces` keep the order in which they were added.

### Project

| Field | Required | Value |
|---|---|---|
| `version` | yes | The version of the format: `1`. See [The version field](#the-version-field). |
| `name` | yes | The name of the project. It must be the file name without `.yaml`. |
| `base` | yes | The [base branch](../concepts.md#base-branch) of every repository that does not set its own. |
| `location` | no | Where the repositories live unless they say otherwise. Left out means this machine. |
| `repos` | no | The registered repositories, by key. Left out means none. |
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
| `env` | no | The env values of `setup` and `run`, by name. A value can read others with templates: see [shared values](../commands/up.md#rules). Set by `octok repo set --env`. |

### Workspaces

An entry of `workspaces` exists only while its workspace has a setting of its own.
It is named like the workspace folder: the branch with `/` replaced by `-`.
`octok rm` removes it with the workspace, including the entry of a [review](../commands/pr-review.md).

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
| `env` | no | Env values of the repository in this workspace: each replaces the value of the repository with the same name, or adds one. Set by `octok repo set --env --branch`. |

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

- **Location:** the file always holds the absolute one.
  The commands also accept `<host>` alone or a relative path: see [Locations and SSH](../commands/locations.md).
- **Key:** the last part of the URL without `.git`, unless `octok repo add --name` gives another.
- **No secrets:** the whole file is plain text, commands included.
  Keep a secret in a `.env` file saved as a [dev change](../commands/dev.md), which never leaves the machine of the repository.

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

`octok project init` and `octok project use` set `default_project`, and `octok project rm` clears it when it removes that project.
A data folder without `config.yaml` is valid: it has no default project.

## The version field

Both kinds of file start with `version: 1`.
`octok` refuses a file with another version before reading anything else, so a file written by a newer `octok` fails with a clear message, not a list of unknown fields.
Nothing is migrated: there is only one version so far.

## Editing by hand

You can edit a project file or `config.yaml` with an editor, and the next command reads the change.

- **The whole file is checked on every read.**
  A value that breaks a rule above makes every command that uses the project fail, and nothing is changed.
  The error names the file when the file cannot be read, and only the value otherwise.
- **Fields are strict:** a typo is an unknown field and an error, never a setting silently ignored.
- **`octok` rewrites the whole file when it changes something.**
  Comments, blank lines and your formatting are lost, and the fields come back in the order of the tables above.
- **A write replaces the file as it was read:** an edit saved while an `octok` command runs can be lost.
- **`octok project ls`** lists every `projects/<project>.yaml` with a valid project name without reading it, so a broken file still shows.
  A file in `projects/` with any other name is ignored.

## Example: invalid files

Each `printf` writes a small invalid project file: an unknown version, a `name` that differs from the file name (as after copying a project file), an unknown field, a location without a path, and a workspace setting for a repository not in the project.
The last two break `config.yaml` instead: a missing `version`, then an unknown one.

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
error: /Users/you/.octotask/projects/shop.yaml: unknown field `repo`, expected one of version, name, base, location, repos, workspaces at line 4, column 1
$ printf 'version: 1\nname: shop\nbase: staging\nlocation: api-dev-1\n' > .octotask/projects/shop.yaml
$ octok status
error: /Users/you/.octotask/projects/shop.yaml: invalid location `api-dev-1`: expected `local` or `<host>:<absolute path>` at line 4, column 11
$ printf 'version: 1\nname: shop\nbase: staging\nworkspaces:\n  feat-x:\n    repos:\n      ghost:\n        run: x\n' > .octotask/projects/shop.yaml
$ octok status
error: repo `ghost` is not registered
$ printf 'oops: 1\n' > .octotask/config.yaml
$ octok project ls
error: /Users/you/.octotask/config.yaml: missing field `version` at line 1, column 1
$ printf 'version: 3\n' > .octotask/config.yaml
$ octok project ls
error: /Users/you/.octotask/config.yaml: unsupported config version 3 (expected 1)
```

## Example: a rewrite drops your comment

Put the copy back, add a comment, and change the project: the comment is gone.

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

## Lock files

`registry.lock` and `cache/<project>/<repo>.lock` are empty files used as locks, so two `octok` commands at once, such as two terminals or an agent next to you, never write the same file together.

- **Who waits:** a command that changes a project file or `config.yaml` holds `registry.lock` until it ends, and an operation on a cache holds the lock of that cache.
  A command that finds a lock taken waits for it.
  Reading takes no lock, and never sees half a file.
- **Leftovers:** the system releases a lock when its process ends, even when killed, so a lock file left behind is not stuck: leave it.
  `octok` never deletes `registry.lock`, and deletes a cache lock only with its cache.
- **`.<file>.tmp-<process id>`** stays only after a process was killed in the middle of a write: it is safe to delete.
- **`cache/<project>/<repo>.tmp-<process id>`** is an interrupted clone, for example by `Ctrl-C`: the next clone of that repository deletes it.

## Related pages

- [Concepts](../concepts.md) defines project, repository, base branch, cache, workspace and location.
- [Projects](../commands/project.md), [Repositories](../commands/repo.md) and [Workspaces](../commands/workspaces.md) describe the commands that write these files.
- [Locations and SSH](../commands/locations.md) explains the data folder on another machine.
- [Dev changes](../commands/dev.md) explains what is in `dev/`.
- [Troubleshooting](../troubleshooting.md) covers the errors of an invalid file.
