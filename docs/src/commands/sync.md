# Syncing

How `octok fetch` updates the clones of every repository of a project and `octok pull` fast-forwards the checkouts of a workspace, and what each one reports.

`octok fetch` brings the [cache](../concepts.md#cache) of every repository of the [project](../concepts.md#project) up to date with its remote.
`octok pull` does a `fetch` and then moves each checkout of a [workspace](../concepts.md#workspace) forward to the branch its remote has, when that is a plain fast-forward.
Both commands print one line per repository and keep going when one repository fails.
Neither one merges, rebases or resets anything.

## `octok fetch`

```text
octok fetch
```

For each repository of the project, in the order of the project, `octok fetch`:

- clones the repository into the cache when the cache is not there, for example one that was deleted by hand, or the one of a repository that moved to another machine;
- otherwise points the clone at the URL that the project file has now, then fetches all the branches of the remote into it.

The fetch prunes: a branch that was deleted on the remote is forgotten by the cache too, and so [`octok status`](status.md) no longer sees it on the remote.
No workspace is touched: `octok fetch` changes the cache and nothing else.
A project without repositories prints nothing and exits with code 0.
It is the one command to run to refresh what `octok status` shows, because `octok status` itself never contacts a git remote.

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

A repository can have two outcomes, and one of them is a failure:

| Line | Meaning |
|---|---|
| `fetched` | The cache is up to date. The text says `fetched` after a clone too: the JSON output says `cloned` or `fetched`. |
| `failed` | The clone or the fetch did not work. The reason is printed on stderr as `error: <repo>: <message>`. |

## `octok pull`

```text
octok pull [<branch>]
```

`octok pull` acts on the workspace of `<branch>`.
Without `<branch>`, it acts on the workspace whose folder contains the current folder, and the project is the one of that workspace, whatever `-p` or the default project say.
Outside a workspace and without a branch, it is refused with exit code 2.
A branch without a workspace is refused with exit code 3.

For each repository of the workspace, `octok pull` fetches its cache and then looks at the checkout:

1. A checkout that is on a branch the remote has is compared with `origin/<branch>`.
   A local branch that was new when the checkout was made, and that was pushed from somewhere else since, starts to follow the remote branch here.
2. When the remote branch is already in the checkout, nothing changes.
3. When the checkout is behind and has no commit of its own, it is fast-forwarded with `git merge --ff-only`.
4. In every other case, the checkout is left as it is and the line says why.

Only a fast-forward is ever done.
`octok pull` never merges, rebases or resets, so it never creates a commit, never rewrites one and never drops a local change.
Uncommitted changes stay in place: if one of them would be overwritten by the fast-forward, git refuses and the repository is reported as `failed`.

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

The command above is run from inside the workspace.
From anywhere else, name the branch: `octok pull feat/cart`.

### What a line says

| Line | What happened |
|---|---|
| `updated` | The checkout was behind its remote branch and was fast-forwarded. |
| `up to date` | The checkout already has every commit of the remote branch. This includes a checkout that is ahead of it. |
| `skipped: branch is not on the remote yet` | The remote of this repository does not have the branch: it was never pushed, or it was deleted there. There is nothing to pull. |
| `skipped: diverged from the remote branch` | The checkout and the remote branch each have commits that the other lacks. Only you can decide between a merge and a rebase, with git, in that checkout. |
| `skipped: detached HEAD` | The checkout is not on a branch. |
| `skipped: no worktree in this workspace` | The repository was registered after the workspace was created: add its checkout with [`octok checkout <branch>`](workspaces.md#complete-a-workspace). |
| `failed` | Something went wrong in this repository, such as an unreachable remote or a fast-forward that git refused. The reason is on stderr as `error: <repo>: <message>`. |

A line that starts with `skipped` is not a failure.
The repository is left as it is, and it does not change the exit code.

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

### Exit codes

Both commands exit with code 0 when every repository went well.
They exit with code 6 when at least one repository is reported `failed`, after they have gone through every repository.
A broken URL is such a case:

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

The reasons and the exit codes are described in [JSON output](../agents/json-output.md).
With `--json`, both commands print one document with the outcome or the error of each repository.

## What is never done

- `octok pull` never merges, rebases or resets, and never creates a commit.
- `octok pull` leaves a checkout that is diverged, on a detached HEAD or on a branch the remote does not have exactly as it is.
- Neither command pushes anything.
- Neither command changes a base branch, a project setting or a workspace.
- `octok fetch` does not need a workspace, and `octok pull` fetches the repositories of its workspace only.

## Related pages

- [Workspaces](workspaces.md) for creating, completing and deleting a workspace.
- [Status](status.md) to see where every checkout stands after a fetch.
- [Troubleshooting](../troubleshooting.md) for an unreachable remote.
