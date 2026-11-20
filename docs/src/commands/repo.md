# Repositories

How to register, list, change and unregister the repositories of a project with `octok repo`, and what a key, an alias and a base branch are for.

A [repository](../concepts.md#repository) is registered in a [project](../concepts.md#project) by its git URL.
`octok repo add` clones it once into the [cache](../concepts.md#cache), and every [workspace](../concepts.md#workspace) is then built from that clone.
All the commands of this page act on the default project, or on the one named with `-p`: see [Projects](project.md).

## Register repositories

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo add git@github.com:acme/shop-agent.git
added `shop-agent` to shop: git@github.com:acme/shop-agent.git
$ octok repo ls
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git
shop-agent                staging  git@github.com:acme/shop-agent.git
```

The repositories are listed in the order they were added, and that is the order every other command uses for them.
Each row shows the key, the alias in parentheses when there is one, the base branch in effect and the URL.
When some repository lives on another machine, a last column shows its [location](../concepts.md#location), `local` for the others.
With `--json`, `octok repo ls` prints the same rows with the key, alias, URL, base branch and location of each repository.
A project without repositories says how to add one:

```console
$ octok project init empty
created project `empty` (base: staging) at /Users/you/.octotask/projects/empty.yaml
`empty` is now the default project
$ octok repo ls
no repos in empty: add one with `octok repo add <url>`
$ octok project use shop
default project: shop
```

## `octok repo add`

```text
octok repo add <url> [--name <key>] [--alias <alias>] [--base <branch>] [--location <location>]
```

Registers the repository and clones it into the cache, with the whole history of its remote.
Nothing is registered if the clone fails, and a clone made by the command is deleted again.
It prints ``added `<repo>` to <project>: <url>``, and `cloned at <location>` when the clone is on another machine.

### The URL

The URL is anything `git clone` takes: `https://`, `http://`, `ssh://`, `git://` and `file://` URLs, the scp-like form `git@github.com:acme/shop-backend.git`, and an absolute path on this machine.
`octok` runs your own `git`, so your credentials, your SSH configuration and your git configuration apply.
For a repository on [another machine](locations.md), the clone is made there, by the `git` of that machine with its own access to the remote, and an absolute path is a path on that machine.
A text that is not one of these, or that has no path after the host, is refused with exit code 2:

```console
$ octok repo add 'not a url'
error: invalid git url `not a url`
$ octok repo add https://github.com/
error: invalid git url `https://github.com/`
```

### The key

The key identifies the repository in the project and is the name of its folder in every workspace.
It is the last part of the path of the URL without `.git`: `git@github.com:acme/shop-frontend.git` gives `shop-frontend`.
`--name <key>` chooses another key.
It is needed when two repositories have the same name, or when the URL gives no usable key:

```console
$ octok repo add git@github.com:acme/shop-frontend.git
error: repo `shop-frontend` is already registered (git@github.com:acme/shop-frontend.git)
choose another name with `--name <key>`
$ octok repo add git@github.com:acme/.git
error: cannot derive a repo key from `git@github.com:acme/.git`: choose one with `--name <key>`
$ octok repo add git@github.com:acme/shop-frontend.git --name web
added `web` to shop: git@github.com:acme/shop-frontend.git
$ octok repo rm web
removed `web` from shop
```

A key, like a project name, has letters, digits, `.`, `_` and `-`, starts with a letter or a digit and has at most 64 characters.
A key that is already taken is refused with exit code 5.
The clone of each repository is separate, even when two keys have the same URL.

### Aliases

`--alias <alias>` gives the repository a shorter name that is accepted wherever the key is: `octok repo set backend ...`, `octok up frontend`, `--pr backend=12`.
Commands print the repository as `key (alias)`.
Keys and aliases share one namespace, so a name always points to one repository: an alias that is the key or the alias of another repository is refused, and so is a key that is the alias of another one.
An alias equal to the key of the repository itself is refused as well, with the same message.
These refusals exit with code 5.

```console
$ octok repo add git@github.com:acme/shop-agent.git --name agent2 --alias frontend
error: alias `frontend` is already used by repo `shop-frontend`
$ octok repo add git@github.com:acme/shop-agent.git --name frontend
error: alias `frontend` is already used by repo `shop-frontend`
```

The second error has the same message, because `frontend` is the alias of `shop-frontend` and the new key would be a second owner of that name.

### The base branch

Each repository starts its new branches from a base branch: its own `--base <branch>`, or else the base branch of the project.
The branch has to exist on the remote.
When it does not, `octok repo add` stops with exit code 3 and registers nothing, unless a person at a terminal confirms:

```console
$ octok repo rm shop-agent
removed `shop-agent` from shop
$ octok repo add git@github.com:acme/shop-agent.git --base develop
error: repo `shop-agent` has no branch `develop` on its remote
$ octok repo add git@github.com:acme/shop-agent.git
added `shop-agent` to shop: git@github.com:acme/shop-agent.git
```

At a terminal, the same case asks first:

```text
branch `develop` is not on the remote of `shop-agent`.
Use it as base anyway? [y/N]
```

Only `y` or `yes` accepts: anything else, an empty answer included, refuses.
Without a terminal, or with `--json`, `octok` refuses without asking.
The same question comes from `octok repo set --base`, and from `--url` or `--location` when the base is missing on the new remote.

### The location

`--location <location>` makes the clone, and later the checkouts, live on another machine, for this repository only.
Without it the repository follows the project.
The forms of a location, what the other machine needs and how a workspace can have its own location are in [Locations and SSH](locations.md).

### Questions at a terminal

At a terminal, `octok repo add` asks for what has no option:

```text
$ octok repo add git@github.com:acme/shop-backend.git
Alias [shop-backend]: backend
Machines in ~/.ssh/config:
  api-dev-1
  bastion
Location [local, project default]: api-dev-1
ssh reaches `api-dev-1` as ubuntu@10.0.0.5, port 22, key /Users/you/.ssh/id_ed25519
Branches on the remote:
  main (default)
  staging
  feat/checkout
  fix/order-totals
Base branch [staging, project default]:
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
cloned at api-dev-1:/home/ubuntu/.octotask
```

- The alias is asked first, with the key as the default shown.
  Enter, or the key itself, saves no alias.
- The location comes next.
  Enter makes the repository follow the project, and the default shown says where that is.
- The base branch is asked last, once the repository is cloned and its branches are known.
  Enter makes the repository follow the base branch of the project.
- The name is never asked: it is the one in the URL, or `--name`.
  A name that cannot be used fails before any question.
- Before the location, `octok` lists the machines that `~/.ssh/config` names, the files it includes covered, at most 15, with `... and N more` after them.
  Patterns such as `*.example.com` are left out.
- A location on another machine is tried at once, and `octok` shows the user, the address, the port and the key files that `ssh` uses for it.
- The branches are listed with the default branch of the remote first, then the staging branch under the names teams use for it (`staging`, `stage`, `stg`, `develop`, `development`, `dev`, `next`, `integration`, `preprod`, `pre-prod`), then the 5 other branches with the most recent commits.
- An answer that cannot be used is explained and asked again.
- With every option given, with `--json`, or without a terminal, nothing is asked and the defaults apply.

## `octok repo set`

```text
octok repo set <repo> [--alias <alias> | --no-alias] [--base <branch> | --no-base]
               [--url <url>] [--location <location> | --no-location]
               [--setup <command> | --no-setup] [--run <command> | --no-run]
octok repo set <repo> (--run <command> | --no-run) --branch <branch>
```

Changes the settings of one repository, given by key or alias.
At least one option is needed, and they can be combined in one command.
An option that gives a value and its `--no-` form cannot be used together: the command line is refused with exit code 2.
A key or alias that is not registered is refused with exit code 3.
`octok repo set` prints ``updated `<repo>` in <project>``, and the label shows the alias after the change.
With `--branch` it prints ``updated `<repo>` in workspace `<workspace>` of <project>`` instead.

| Option | What it does |
|---|---|
| `--alias <alias>` | Sets or changes the alias. It follows the rules of the shared namespace above. |
| `--no-alias` | Removes the alias. |
| `--base <branch>` | Sets the base branch of this repository. |
| `--no-base` | Goes back to the base branch of the project. |
| `--url <url>` | Changes the URL and fetches the clone from it. |
| `--location <location>` | Moves the repository to another machine. |
| `--no-location` | Goes back to the location of the project. |
| `--setup <command>`, `--no-setup` | Sets or removes the command that prepares a checkout. See [Running](up.md). |
| `--run <command>`, `--no-run` | Sets or removes the command that starts the repository. See [Running](up.md). |
| `--branch <branch>` | With `--run` or `--no-run` only: changes the run command in the workspace of that branch alone. See [Running](up.md). |

```console
$ octok repo set agent --alias agent
error: repo `agent` is not registered
$ octok repo set shop-agent --alias agent
updated `shop-agent (agent)` in shop
$ octok repo set agent --base main
updated `shop-agent (agent)` in shop
$ octok repo ls
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git
shop-agent (agent)        main     git@github.com:acme/shop-agent.git
$ octok repo set agent --no-base --no-alias
updated `shop-agent` in shop
$ octok repo set shop-agent --base develop
error: repo `shop-agent` has no branch `develop` on its remote
```

A change of base, URL or location is checked against the remote first, so the clone is fetched.
If that fails, the project file is left as it was, and so is the URL the clone fetches from.
A base branch that the remote does not have is refused with exit code 3, as with `octok repo add`.
A rerun of the same command changes nothing more.

### Moving a repository

A repository changes location only while no workspace has a checkout of it, because the checkouts would be left behind: otherwise the command is refused with exit code 5, as `octok repo rm` is.
Its clone is deleted on the old machine, together with the saved [dev changes](dev.md), and made again on the new one.

## `octok repo rm`

```text
octok repo rm <repo>
```

Unregisters the repository, given by key or alias, and deletes its clone and its saved dev changes, on the machine where they are.
Its settings in the workspaces are forgotten too.
It never touches the remote.

It is refused, with exit code 5, while a workspace has a checkout of the repository, because the commits of those checkouts live in the clone.
The error names the workspaces:

```console
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok repo rm backend
error: repo `shop-backend` is used by workspaces: feat-cart
$ octok rm feat/cart
removed workspace `feat-cart` from shop
$ octok repo rm backend
removed `shop-backend` from shop
```

[`octok rm`](workspaces.md#delete-a-workspace) removes a workspace.
Run `octok repo add` again to bring the repository back: it is cloned again.

## Related pages

- [Projects](project.md) for the default base branch and location.
- [Workspaces](workspaces.md) for the checkouts of a repository.
- [Running](up.md) for the `setup` and `run` commands.
- [Configuration files](../reference/configuration.md) for how a repository is stored.
