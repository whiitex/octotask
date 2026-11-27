# Projects

How to create, list, select, change and delete the projects of `octok`, the groups of repositories that every other command acts on.

## How to use it

1. Create a project with `octok project init <name>`: it becomes the default.
2. Register its repositories with [`octok repo add <url>`](repo.md).
3. Switch the default with `octok project use <name>`, or name another project for one command with `-p <name>`.
4. Delete it with `octok project rm <name>`, after [`octok rm <branch>`](workspaces.md) of its workspaces.

## Options

| Command | What it does |
|---|---|
| `octok project init <name> [--base <branch>] [--location <location>]` | Creates `~/.octotask/projects/<name>.yaml` and makes it the default. The base defaults to `staging`, the location to this machine, `local`. |
| `octok project ls` | Lists the projects alphabetically, `*` on the default. |
| `octok project use <name>` | Makes `<name>` the default. |
| `octok project set --base <branch>` | Changes the base branch. |
| `octok project set --location <location>` | Moves the repositories to that [machine](locations.md). `--no-location` brings them back here. |
| `octok project rm <name> [--force]` | Deletes the project. `--force` deletes its workspaces too. |
| `-p <project>`, `--project <project>` | Acts on another project for one command. It goes before or after the command, as `--json` does. |

## Rules

- **`init` creates only the file,** no clone or folder, and never overwrites a project.
  It does not check the base branch, since there is no repository yet: `octok repo add` does.
  With `--location` it reaches the machine first, so an unreachable one leaves nothing behind.
- **Names:** letters, digits, `.`, `_` and `-`, starting with a letter or a digit, at most 64 characters.
  The base must be a branch name git accepts.
- **Questions:** at a terminal, `init` asks for each setting without an option, default in brackets, listing the `Host` names of `~/.ssh/config`.
  Enter keeps the default, and an unusable answer is asked again.
  Nothing is asked without a terminal, with `--json`, or with both options.
- **Default project:** the one made default last.
  Every command acts on it or on `-p`, except `octok project init`, `ls`, `use`, `rm` and `octok docs`, which ignore `-p`.
  [`octok pull`](sync.md), [`octok up`](up.md) and [`octok dev save`](dev.md) without a branch act on the workspace of the current folder, whatever `-p` says.
- **`set`** needs one option at least; with `--base` and `--location` the base changes first, and stays if the location is refused.
- **`set --base`** leaves alone the repositories with their own base.
  It warns on stderr about fetched repositories whose remote lacked their base at the last fetch, without contacting the remotes, and changes it anyway.
- **`set --location`** is refused while the project has workspaces.
  The clones of repositories without their own location are deleted, with their saved [dev changes](dev.md), and made again on the new machine.
  It prints one line per repository, as [`octok fetch`](sync.md) does: `fetched` for a clone too, which the JSON output tells apart.
- **`rm`** deletes, on every machine of the project, its file, clones, workspaces and saved dev changes, without asking.
  It removes the `octok` folder of another machine when this was its last project, and never touches other projects or the remotes.
  Afterwards, if it was the default, there is no default project.
- **`rm` with workspaces** is refused, since they may hold work found nowhere else; `--force` deletes them with their uncommitted changes and unpushed commits.
- **`rm` of a broken project:** clones go before the file, so after a failure the project stays and `octok fetch` clones again.
  A file that cannot be read, such as one mistyped by hand, is still deleted, cleaning only this machine.

| Exit code | When |
|---|---|
| 2 | An invalid name or base branch, or no `-p` and no default project. |
| 3 | No such project, for `-p` or `use`: ``error: project `nope` not found``. |
| 5 | `init` of an existing name, or `set --location` or `rm` while the project has workspaces. |
| 6 | `set --location` could not clone a repository. |

## Example: create projects and choose the default

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

## Example: act on another project

`-p blog` leaves the default on `shop`:

```console
$ octok repo add git@github.com:acme/shop-frontend.git
added `shop-frontend` to shop: git@github.com:acme/shop-frontend.git
$ octok -p blog repo ls
no repos in blog: add one with `octok repo add <url>`
$ octok repo ls
shop-frontend  staging  git@github.com:acme/shop-frontend.git
```

With no `-p` and no default project:

```console
$ OCTOTASK_HOME=/Users/you/empty octok status
error: no project given and no default project set (use `octok project use <name>`)
```

## Example: what `init` refuses

```console
$ octok project init -- -bad
error: invalid project name `-bad`: must start with a letter or digit
$ octok project init shop
error: project `shop` already exists
```

## Example: a project on another machine

```console
$ octok project init remote --location api-dev-1
created project `remote` (base: staging) at /Users/you/.octotask/projects/remote.yaml
its repos live at api-dev-1:/home/ubuntu/.octotask
`remote` is now the default project
$ octok project use shop
default project: shop
```

## Example: questions at a terminal

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

## Example: change the base branch

The remote of `shop-frontend` has no `develop`:

```console
$ octok project set --base develop
warning: branch `develop` is not on the remote of: shop-frontend
base of `shop` is now `develop`
$ octok project set --base staging
base of `shop` is now `staging`
```

## Example: move the repositories

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

## Example: delete a project

```console
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok project rm shop
error: project `shop` still has workspaces: feat-cart
remove them first with `octok rm <branch>`
```

```console
$ octok project rm shop --force
removed project `shop`
no default project: choose one with `octok project use <name>`
```

## Related pages

- [Repositories](repo.md) for the repositories of a project.
- [Locations and SSH](locations.md) for `--location`.
- [Configuration files](../reference/configuration.md) for the file of a project.
- [JSON output](../agents/json-output.md) for the documents and the exit codes.
