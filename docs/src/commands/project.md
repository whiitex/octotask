# Projects

How to create, list, select, change and delete the projects of `octok`, the groups of repositories that every other command acts on.

A [project](../concepts.md#project) is a named group of repositories with a default [base branch](../concepts.md#base-branch) and, optionally, a default [location](../concepts.md#location).
Every other command acts on one project: the default one, or the one you name with `-p`.
The commands of this page create a project, choose which one is the default, change its settings and delete it.
The repositories of a project are managed with [`octok repo`](repo.md).

## Create a project and use it

`octok project init <name>` creates the project and makes it the default:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok project init blog --base main
created project `blog` (base: main) at /Users/you/.octotask/projects/blog.yaml
`blog` is now the default project
$ octok project ls
* blog
  shop
```

`octok project ls` marks the default project with `*`.
It shows the project that was made the default last, so after the second `init` the default is `blog`.
`octok project use <name>` selects another one:

```console
$ octok project use shop
default project: shop
$ octok project ls
  blog
* shop
```

Without projects, `octok project ls` says how to create one:

```console
$ OCTOTASK_HOME=/Users/you/empty octok project ls
no projects yet: create one with `octok project init <name>`
```

The project names are listed in alphabetical order.

## The default project and `-p`

Every command except `octok project init`, `ls`, `use`, `rm` and `octok docs` acts on the default project.
The global option `-p <project>` (or `--project <project>`) acts on another project for one command, without changing the default.
It goes before or after the command, as with `--json`:

```console
$ octok repo add git@github.com:acme/shop-frontend.git
added `shop-frontend` to shop: git@github.com:acme/shop-frontend.git
$ octok -p blog repo ls
no repos in blog: add one with `octok repo add <url>`
$ octok repo ls
shop-frontend  staging  git@github.com:acme/shop-frontend.git
```

Without `-p` and without a default project, a command is refused with exit code 2:

```console
$ OCTOTASK_HOME=/Users/you/empty octok status
error: no project given and no default project set (use `octok project use <name>`)
```

A project that does not exist is refused with exit code 3: `octok -p nope status` prints ``error: project `nope` not found``.
`octok project init`, `ls`, `use` and `rm` take the project as an argument or need none, so they ignore `-p`.
`octok project use` refuses a name that is not a project, with exit code 3.
[`octok pull`](sync.md#octok-pull), [`octok up`](up.md) and [`octok dev save`](dev.md) without a branch act on the workspace that contains the current folder, and so on its project, whatever `-p` and the default project say.

## `octok project init`

```text
octok project init <name> [--base <branch>] [--location <location>]
```

Creates the file of the project in `~/.octotask/projects/<name>.yaml` and makes the project the default.
It does not create any repository, clone or folder.

| Option | What it sets |
|---|---|
| `--base <branch>` | The default base branch of the repositories. The default is `staging`. |
| `--location <location>` | The machine where the repositories live, unless one of them says otherwise. The default is this machine, `local`. See [Locations and SSH](locations.md). |

The name has to be usable as a folder name: letters, digits, `.`, `_` and `-`, starting with a letter or a digit, at most 64 characters.
The base branch has to be a name that git accepts for a branch.
Otherwise the command is refused with exit code 2 and nothing is created.

```console
$ octok project init -- -bad
error: invalid project name `-bad`: must start with a letter or digit
$ octok project init shop
error: project `shop` already exists
```

An existing name is refused with exit code 5: `init` never overwrites a project.
The base branch is a default: it is not checked against any remote, because the project has no repository yet.
`octok repo add` checks it for each repository.

With `--location`, `octok` connects to the machine once to find its home folder, before it creates anything, so a machine that cannot be reached leaves nothing behind:

```console
$ octok project init remote --location api-dev-1
created project `remote` (base: staging) at /Users/you/.octotask/projects/remote.yaml
its repos live at api-dev-1:/home/ubuntu/.octotask
`remote` is now the default project
$ octok project use shop
default project: shop
```

### Questions at a terminal

At a terminal, `octok project init` asks for each setting that has no option, and shows the default between brackets:

```text
$ octok project init shop
Base branch [staging]:
Machines in ~/.ssh/config:
  api-dev-1
  bastion
Location [local]:
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
```

Press Enter to keep the default.
An answer that cannot be used, such as a branch name that git refuses or a machine that cannot be reached, is explained and asked again.
The list of machines is the `Host` names of `~/.ssh/config`: see [Locations and SSH](locations.md).
Without a terminal, for example in a script, nothing is asked and the defaults apply.
Nothing is asked either with `--json`, or when `--base` and `--location` are both given.

## `octok project set`

```text
octok project set [--base <branch>] [--location <location> | --no-location]
```

Changes the settings of the default project, or of the one named with `-p`.
At least one option is needed.
The options can be combined.
With `--base` and `--location` together, the base branch is changed first: if the location is then refused, the new base branch stays.

`--base <branch>` changes the base branch of the project.
The repositories that have their own base branch keep it.
`octok` warns, on stderr, about the repositories whose remote does not have the new branch at the last fetch.
It uses what is on disk and does not contact the remotes, and the change is made all the same:

```console
$ octok project set --base develop
warning: branch `develop` is not on the remote of: shop-frontend
base of `shop` is now `develop`
$ octok project set --base staging
base of `shop` is now `staging`
```

The warning names the repositories that were fetched and whose base branch is not on their remote.
A repository with its own base branch is checked against that one.

`--location <location>` changes the machine where the repositories of the project live, and `--no-location` goes back to this machine.
It is refused, with exit code 5, while the project has workspaces, because their checkouts would be left behind: remove them first with [`octok rm`](workspaces.md#delete-a-workspace).
The clones of the repositories that move are deleted where they were, together with their saved [dev changes](dev.md), and cloned again on the new machine.
Repositories that have their own location do not move.
The command prints one line per repository, as [`octok fetch`](sync.md) does, and exits with code 6 if one of them could not be cloned:

```console
$ octok repo add git@github.com:acme/shop-backend.git
added `shop-backend` to shop: git@github.com:acme/shop-backend.git
$ octok project set --location api-dev-1
repos of `shop` now live at api-dev-1:/home/ubuntu/.octotask
shop-frontend  fetched
shop-backend   fetched
$ octok project set --no-location
repos of `shop` now live at local
shop-frontend  fetched
shop-backend   fetched
```

The word `fetched` is printed whether the repository was cloned or fetched; the JSON output tells them apart.
`--location` and `--no-location` cannot be given together.

## `octok project rm`

```text
octok project rm <name> [--force]
```

Deletes a project: its file, the clones of its repositories, its workspaces and the dev changes saved for its repositories.
It does this on every machine that the project uses, and if the project was the default, there is no default project afterwards.
When the project was the last one on another machine, the whole `octok` folder of that machine goes too.
Folders of other projects, and the remotes, are never touched.

`octok project rm` is refused, with exit code 5, while the project has workspaces, because they may hold work that exists nowhere else:

```console
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok project rm shop
error: project `shop` still has workspaces: feat-cart
remove them first with `octok rm <branch>`
```

Remove the workspaces first, with [`octok rm`](workspaces.md#delete-a-workspace), which also checks that no work is lost.
`--force` deletes the workspaces too, and with them the uncommitted changes and the commits that were never pushed.

```console
$ octok project rm shop --force
removed project `shop`
no default project: choose one with `octok project use <name>`
```

Without workspaces, `octok project rm <name>` deletes the project at once, and does not ask for confirmation.
The clones are deleted before the project file: if that fails, the project is still there and `octok fetch` clones the repositories again.
A project file that cannot be read, for example one edited by hand with a mistake, is still deleted: only this machine is cleaned then.

## Related pages

- [Repositories](repo.md) for the repositories of a project.
- [Locations and SSH](locations.md) for `--location`.
- [Configuration files](../reference/configuration.md) for the file of a project.
- [JSON output](../agents/json-output.md) for the documents and the exit codes.
