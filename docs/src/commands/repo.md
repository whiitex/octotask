# Repositories

How to register, list, change and unregister the repositories of a project with `octok repo`, and what a key, an alias and a base branch are for.

## How to use it

1. Register each repository by its git URL: `octok repo add <url> --alias <alias>`.
   It is cloned once into the cache, and every workspace is built from that clone.
2. Check what is registered with `octok repo ls`.
3. Change a setting with `octok repo set <repo> <option>...`, such as the base branch or the [run command](up.md).
4. Unregister it with `octok repo rm <repo>`.

Every command acts on the default project or on `-p`: see [Projects](project.md).
A `<repo>` is a key or an alias.

## Options

| Command or option | What it does |
|---|---|
| `octok repo add <url>` | Registers and clones the repository. Takes `--name`, `--alias`, `--base` and `--location`. |
| `octok repo ls` | Lists the repositories. |
| `octok repo set <repo> <option>...` | Changes one repository. Takes every option below except `--name`. |
| `octok repo rm <repo>` | Unregisters the repository and deletes its clone. |
| `--name <key>` | Chooses the key instead of the one of the URL. |
| `--alias <alias>`, `--no-alias` | Sets or removes the alias. |
| `--base <branch>`, `--no-base` | Sets the base branch, or goes back to the one of the project. |
| `--location <location>`, `--no-location` | Puts the clone and the checkouts on another [machine](locations.md), or goes back to the location of the project. |
| `--url <url>` | Changes the URL and fetches the clone from it. |
| `--setup <command>`, `--no-setup` | Sets or removes the command that prepares a checkout. See [Running](up.md). |
| `--run <command>`, `--no-run` | Sets or removes the command that starts the repository. |
| `--env <name>=<value>`, `--no-env <name>` | Sets or removes an env value of the setup and run commands. Both can be repeated. |
| `--branch <branch>` | With `--run`, `--no-run`, `--env` and `--no-env` only: changes them in the workspace of that branch alone. |

## Rules

- **Order:** every command lists and handles the repositories in the order they were added.
- **`repo ls`** shows the key, the alias in parentheses, the base branch in effect and the URL, plus a location column (`local` or a [location](locations.md)) when some repository lives elsewhere.
  `--json` gives the key, alias, URL, base branch and location.
- **URL:** anything `git clone` takes: `https://`, `http://`, `ssh://`, `git://`, `file://`, the scp-like `git@github.com:acme/shop-backend.git`, or an absolute path.
  `octok` runs your own `git`, with your credentials and configuration.
  On another machine, that machine's `git` clones with its own access, and a path is a path there.
- **Clone:** `add` clones the whole history, one clone per key even for the same URL.
  If it fails, nothing is registered and the partial clone is deleted.
  It prints ``added `<repo>` to <project>: <url>``, and `cloned at <location>` on another machine.
- **Key:** it names the repository and its folder in every workspace.
  It is the last part of the URL path without `.git`; `--name` picks another, when two names clash or the URL gives none.
  It follows the rules of a project name: letters, digits, `.`, `_` and `-`, starting with a letter or a digit, at most 64 characters.
- **Alias:** a shorter name, accepted wherever the key is (`octok up frontend`, `--pr backend=12`), and printed as `key (alias)`.
  Keys and aliases share one namespace, so an alias or a key already used by any repository is refused, and so is an alias equal to its own key.
- **Base branch:** new branches start from the `--base` of the repository, or else from the one of the project.
  `add` and `set` refuse a base the remote lacks, also after `--url` or `--location`.
  At a terminal they ask ``branch `develop` is not on the remote of `shop-agent`. Use it as base anyway? [y/N]`` instead, and only `y` or `yes` accepts; with `--json` they never ask.
- **Location:** without `--location`, a repository follows the project: see [Locations and SSH](locations.md) for the forms and for the location of one workspace.
- **Questions:** at a terminal, `add` asks for each setting without an option, as in the example below; Enter, or the key, saves no alias, and Enter follows the project for the location and the base.
  The key is never asked, and an unusable one fails before any question.
  The machines listed are the `Host` names of `~/.ssh/config` and its includes, at most 15 then `... and N more`, without patterns such as `*.example.com`.
  A remote machine is tried at once, showing the user, address, port and key files `ssh` uses.
  The branches listed are the default of the remote, the staging branch (`staging`, `stage`, `stg`, `develop`, `development`, `dev`, `next`, `integration`, `preprod`, `pre-prod`) and the 5 other most recent ones.
  An unusable answer is asked again, and nothing is asked with every option, `--json`, or no terminal.
- **`set`** needs one option at least, and refuses an option with its own `--no-` form, or the same env name set and removed.
  It prints ``updated `<repo>` in <project>`` with the new alias, or ``updated `<repo>` in workspace `<workspace>` of <project>`` with `--branch`.
  A change of base, URL or location fetches the clone first, and if that fails nothing changes.
- **Moving** deletes the clone on the old machine, with its saved [dev changes](dev.md), and clones it again on the new one.
- **`set --location` and `rm`** are refused while a workspace has a checkout, whose commits live in the clone; `rm` names the workspaces.
- **`rm`** deletes the clone and saved dev changes wherever they are, and forgets the settings of the repository in the workspaces.
  It never touches the remote, and `octok repo add` brings the repository back.

| Exit code | When |
|---|---|
| 2 | An invalid URL, a URL that gives no key, or an option with its `--no-` form. |
| 3 | A `<repo>` that is not registered, or a base branch the remote lacks. |
| 5 | A key or an alias already used, or `set --location` or `rm` while a workspace has a checkout. |

## Example: register repositories

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

## Example: what `add` refuses

```console
$ octok repo add 'not a url'
error: invalid git url `not a url`
$ octok repo add https://github.com/
error: invalid git url `https://github.com/`
```

A key taken, or missing from the URL, needs `--name`:

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

`frontend` is the alias of `shop-frontend`:

```console
$ octok repo add git@github.com:acme/shop-agent.git --name agent2 --alias frontend
error: alias `frontend` is already used by repo `shop-frontend`
$ octok repo add git@github.com:acme/shop-agent.git --name frontend
error: alias `frontend` is already used by repo `shop-frontend`
```

```console
$ octok repo rm shop-agent
removed `shop-agent` from shop
$ octok repo add git@github.com:acme/shop-agent.git --base develop
error: repo `shop-agent` has no branch `develop` on its remote
$ octok repo add git@github.com:acme/shop-agent.git
added `shop-agent` to shop: git@github.com:acme/shop-agent.git
```

## Example: questions at a terminal

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

## Example: change a repository

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

## Example: unregister a repository

`feat-cart` has a checkout of `shop-backend`:

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

## Related pages

- [Projects](project.md) for the default base branch and location.
- [Workspaces](workspaces.md) for the checkouts of a repository.
- [Running](up.md) for the `setup` and `run` commands.
- [Configuration files](../reference/configuration.md) for how a repository is stored.
