# Syncing

How `octok fetch` updates the clones of every repository of a project and `octok pull` fast-forwards the checkouts of a workspace, and what each one reports.

## How to use it

1. Run `octok fetch` to refresh the [cache](../concepts.md#cache) of every repository, so [`octok status`](status.md) shows the remotes as they are now.
2. Run `octok pull` inside a [workspace](../concepts.md#workspace), or `octok pull <branch>` from anywhere, to fast-forward its checkouts.
3. Settle a `skipped: diverged from the remote branch` yourself, with git, in that checkout.

## Options

| Command | What it does |
|---|---|
| `octok fetch` | Clones or fetches the cache of every repository of the project. |
| `octok pull [<branch>]` | Fetches the repositories of the workspace, then fast-forwards each checkout. |

## Rules

- **One line per repository:** both commands go on after a failure and print its reason on stderr as `error: <repo>: <message>`.
- **`octok fetch` touches only the cache:** it clones a missing one, such as one deleted by hand or after a move, and otherwise points it at the URL of the project file and fetches every branch.
  A project without repositories prints nothing.
- **Pruning:** a branch deleted on the remote leaves the cache too, so `octok status` stops showing it.
- **`octok status` never contacts a remote:** `octok fetch` is how to refresh it.
- **Which workspace:** without `<branch>`, `octok pull` uses the workspace of the current folder and its project, whatever `-p` or the default project say.
- **Only fast-forwards:** a checkout behind `origin/<branch>` with no commit of its own moves with `git merge --ff-only`, and any other is left alone.
  `octok pull` never merges, rebases, resets or creates a commit.
- **Uncommitted changes** stay: if the fast-forward would overwrite one, git refuses and the line says `failed`.
- **Pushed elsewhere:** a branch that was new here and was pushed from somewhere else starts to follow the remote branch.
- **`skipped` is not a failure:** the checkout stays as it is, and the exit code does not change.
- **Nothing else changes:** neither command pushes or changes a base branch, a setting or a workspace, and `octok pull` fetches only its own repositories.
- **`--json`** prints one document with the outcome or error of each repository, `cloned` or `fetched` for a fetch: see [JSON output](../agents/json-output.md).

| Line | Meaning |
|---|---|
| `fetched` | `octok fetch` updated or cloned the cache. |
| `updated` | The checkout was behind and was fast-forwarded. |
| `up to date` | The checkout has every commit of the remote branch, or more. |
| `skipped: branch is not on the remote yet` | The branch was never pushed, or was deleted on the remote. |
| `skipped: diverged from the remote branch` | Both sides have commits the other lacks. |
| `skipped: detached HEAD` | The checkout is not on a branch. |
| `skipped: no worktree in this workspace` | The repository came after the workspace: add it with [`octok checkout <branch>`](workspaces.md#example-complete-a-workspace). |
| `failed` | For example an unreachable remote, or a fast-forward that git refused. |

| Exit code | When |
|---|---|
| 0 | No repository `failed`. |
| 2 | `octok pull` outside a workspace, without a branch. |
| 3 | `octok pull <branch>` for a branch without a workspace. |
| 6 | At least one repository `failed`, reported after all the others. |

## Example: fetch a project

Every repository of a new project is fetched:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo add git@github.com:acme/shop-agent.git --alias agent
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
$ octok fetch
shop-frontend (frontend)  fetched
shop-backend (backend)    fetched
shop-agent (agent)        fetched
```

## Example: pull the commit of a teammate

A teammate pushes to `feat/cart` after you, and `octok pull` brings the commit in:

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ cd shop-backend
$ echo 'print(1)' > cart.py
$ git add cart.py
$ git commit --quiet --message 'Add cart endpoint'
$ git push --quiet
$ cd "$HOME"
$ git clone --quiet git@github.com:acme/shop-backend.git teammate
$ cd teammate
$ git switch --quiet feat/cart
$ echo 'print(2)' > checkout.py
$ git add checkout.py
$ git commit --quiet --message 'Add checkout endpoint'
$ git push --quiet
$ cd "$HOME/.octotask/workspaces/shop/feat-cart"
$ octok pull
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    updated
shop-agent (agent)        skipped: branch is not on the remote yet
$ octok pull
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    up to date
shop-agent (agent)        skipped: branch is not on the remote yet
$ git -C shop-backend log --oneline -1
5496088 Add checkout endpoint
```

## Example: a diverged checkout

Both sides commit, so the checkout is left alone, with exit code 0:

```console
$ cd "$HOME/.octotask/workspaces/shop/feat-cart/shop-backend"
$ echo 'print(3)' > local.py
$ git add local.py
$ git commit --quiet --message 'Add a local file'
$ cd "$HOME/teammate"
$ echo 'print(4)' > remote.py
$ git add remote.py
$ git commit --quiet --message 'Add a remote file'
$ git push --quiet
$ cd "$HOME/.octotask/workspaces/shop/feat-cart"
$ octok pull; echo "exit code $?"
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    skipped: diverged from the remote branch
shop-agent (agent)        skipped: branch is not on the remote yet
exit code 0
```

## Example: a broken URL

A wrong URL fails one repository, the others still fetch, and the exit code is 6:

```console
$ sed -i.bak 's#acme/shop-agent#acme/nope#' "$HOME/.octotask/projects/shop.yaml"
$ octok fetch; echo "exit code $?"
shop-frontend (frontend)  fetched
shop-backend (backend)    fetched
shop-agent (agent)        failed
error: shop-agent (agent): `git fetch --prune --quiet origin` failed in /Users/you/.octotask/cache/shop/shop-agent.git (exit status: 128)
fatal: 'git@github.com:acme/nope.git' does not appear to be a git repository
fatal: Could not read from remote repository.

Please make sure you have the correct access rights
and the repository exists.
exit code 6
```

## Related pages

- [Workspaces](workspaces.md) for creating, completing and deleting a workspace.
- [Status](status.md) to see where every checkout stands after a fetch.
- [Troubleshooting](../troubleshooting.md) for an unreachable remote.
