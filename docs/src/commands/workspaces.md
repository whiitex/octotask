# Workspaces

How `octok branch`, `octok checkout` and `octok rm` create, complete and delete a workspace: one folder with a checkout of every repository, all on the same branch.

## How to use it

1. Start a new branch in every repository with `cd "$(octok branch <branch>)"`: `octok` prints the path, and the shell does the `cd`.
2. Publish it with a plain `git push` in each checkout.
3. Open a branch someone already pushed with `cd "$(octok checkout <branch>)"`.
4. Delete the workspace with `octok rm <branch>`.

For pull requests, see [Pull request review](pr-review.md).

## Options

| Command or option | What it does |
|---|---|
| `octok branch <branch>` | Creates the workspace of a new branch from the base of each repository. |
| `octok checkout <branch>` | Prints the path of the workspace of a pushed branch, creating or completing it first. |
| `octok checkout -b <branch>` | The same as `octok branch <branch>`. |
| `--location <location>` | With `branch` or `checkout`: the whole workspace lives on that [machine](locations.md), whatever the repositories and the project say. |
| `--allow-stale-dev` | With `branch` or `checkout`: creates the workspace even if a [dev change](dev.md) no longer applies, leaving that file as it is in git. |
| `octok rm <branch> [--force]` | Deletes the workspace. `--force` deletes it even when work would be lost. |

## Rules

- **Layout:** each checkout is a `git worktree` of the cache, in `~/.octotask/workspaces/<project>/<workspace>/<repo>`, so a workspace is instant and several can be open at once.
- **Name:** the branch with every `/` replaced by `-`, as [`octok status`](status.md) shows it: `feat/cart` lives in `feat-cart`.
  Commands take the branch.
  Two branches with the same name, such as `feat/cart` and `feat-cart`, cannot both have a workspace.
- **Output:** the path goes to stdout, as `<host>:<path>` when the workspace has no folder here, and the rest to stderr.
  On stderr, `created workspace` or `completed workspace` comes first, then the repositories under `branch from the remote`, `new branch from the base` or `branch restored from the cache`.
  When every checkout is new from the base only the first line is printed, and nothing for a complete workspace.
  A checkout on another machine adds a line with its path there.
- **A new branch** starts at the tip of the base branch on the remote, fetched first: the base of the repository, or else of the project (see [Repositories](repo.md)).
  It does not follow the base, and stays local until pushed.
- **Publishing:** a plain `git push` sets the upstream, no `-u origin <branch>` needed, and `git pull` works as in any clone.
  Until then `octok status` marks the branch `[not on the remote]`.
- **`branch` refuses a pushed branch** on any remote of the project, so a new name never lands on someone else's work.
- **`checkout`** gives each repository that has the branch on its remote a local branch at that commit, following it for `git pull`, `git push` and [`octok pull`](sync.md).
  The others get a new branch from their base, so [`octok up`](up.md) can start everything, and `octok pull` follows it once pushed there.
  A branch on no remote is refused: create it with `octok checkout -b <branch>`.
- **One branch, one checkout:** git holds a branch in one checkout, so a branch checked out in another workspace, such as one of [`octok pr review`](pr-review.md), is refused with its folder.
- **Completing:** a rerun adds only the missing checkouts: of a repository registered later, of an interrupted creation, or of a folder deleted by hand, which comes back with its commits.
  `checkout` always completes; `branch` does unless a repository it adds has the branch on its remote.
  On a complete workspace, `checkout` prints the path without a fetch, and `branch` is refused.
- **Dev changes:** each new checkout gets the saved [dev changes](dev.md) of its repository.
  If one no longer applies, the command names the file and creates nothing, unless `--allow-stale-dev`.
- **All or nothing:** a failed checkout removes the checkouts and branches the command made.
  A base missing on a remote, a project without repositories, or a branch name git refuses (checked before any fetch) creates nothing.
- **`rm` refuses to lose work,** listing what only the workspace holds: uncommitted or untracked changes, commits on no branch of the remote at the last fetch or push (detached HEAD included), an unreadable checkout, or files that are not a checkout.
  `--force` deletes it all anyway.
- **`rm` does not check** files git ignores, such as `.env`, or the dev changes in the checkouts: it deletes them without a warning, and the next workspace gets the dev changes again.
- **`rm` deletes** the folder, then the local branch of the workspace in each repository, the branch the checkout had if different, and the branches saved for a pull request review.
  It forgets the location and run commands of the workspace in the project file, and cleans the caches of a workspace whose folder was deleted by hand.
- **`rm` keeps** a branch checked out in another workspace, the branches of the remotes, the base branches, the cache and the repository.
- **`--json`** prints the project, the workspace, its path and, per repository, its checkout path and outcome: see [JSON output](../agents/json-output.md).

| Exit code | When |
|---|---|
| 2 | A branch name git does not accept. |
| 3 | A base branch missing on a remote, a project without repositories, `checkout` of a branch on no remote, or `rm` of a branch without a workspace. |
| 4 | `rm` found work that would be lost. |
| 5 | `branch` of a pushed branch or an existing workspace, a name taken by another branch, a branch checked out elsewhere, or a dev change that no longer applies. |

## Example: create the workspace of a new branch

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ ls
shop-backend
shop-frontend
$ cd shop-backend
$ git branch --show-current
feat/cart
$ git log --oneline -1
de08597 Fix order totals
```

`feat-cart` belongs to `feat/cart`, and a rerun of `branch` is refused:

```console
$ octok branch feat-cart
error: workspace `feat-cart` is the one of the branch `feat/cart`, whose name gives the same folder
remove it with `octok rm feat/cart`, or use another branch name
$ octok branch feat/cart
error: workspace `feat-cart` already exists at /Users/you/.octotask/workspaces/shop/feat-cart
```

## Example: publish the branch

```console
$ cd "$HOME/.octotask/workspaces/shop/feat-cart/shop-backend"
$ echo 'print(2)' > cart.py
$ git add cart.py
$ git commit --quiet --message 'Add cart endpoint'
$ git push
To git@github.com:acme/shop-backend.git
 * [new branch]      feat/cart -> feat/cart
branch 'feat/cart' set up to track 'origin/feat/cart'.
$ git status --short --branch
## feat/cart...origin/feat/cart
```

## Example: open a branch that already exists

`feat/login` is on the remotes of the frontend and the backend only:

```console
$ octok branch feat/login
error: branch `feat/login` already exists on the remote of: shop-frontend, shop-backend
check it out with `octok checkout feat/login`, or choose another name
$ octok repo add git@github.com:acme/shop-agent.git --alias agent
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
$ cd "$(octok checkout feat/login)"
created workspace `feat-login` in shop
  branch from the remote: shop-frontend, shop-backend
  new branch from the base: shop-agent
$ cd shop-frontend
$ git log --oneline -1
1695c74 Add login form
$ git status --short --branch
## feat/login...origin/feat/login
```

```console
$ octok checkout feat/nothing
error: no repo has a branch `feat/nothing` on its remote: create it with `octok checkout -b feat/nothing`
```

## Example: complete a workspace

`shop-agent` was added after `feat-cart`, and the second `checkout` only prints the path:

```console
$ cd "$(octok checkout feat/cart)"
completed workspace `feat-cart` in shop
  new branch from the base: shop-agent
$ ls
shop-agent
shop-backend
shop-frontend
$ cd "$(octok checkout feat/cart)"
```

## Example: delete a workspace

```console
$ cd "$(octok branch feat/search)/shop-backend"
created workspace `feat-search` in shop
$ echo 'print(1)' > scratch.py
$ git add scratch.py
$ git commit --quiet --message 'Add scratch'
$ echo 'x = 1' > draft.py
$ echo notes > ../notes.txt
$ octok rm feat/search
error: workspace `feat-search` has work that would be lost:
  shop-backend: uncommitted changes
  shop-backend: 1 unpushed commit on `feat/search`
  other files in the workspace: notes.txt
remove it anyway with `octok rm --force <branch>`
$ cd "$HOME"
$ octok rm feat/search --force
removed workspace `feat-search` from shop
```

```console
$ octok rm feat/search
error: no workspace for `feat/search`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch
```

## Example: JSON output

```console
$ octok branch feat/json --json
{"schema":1,"project":"shop","workspace":"feat-json","path":"/Users/you/.octotask/workspaces/shop/feat-json","repos":[{"key":"shop-frontend","path":"/Users/you/.octotask/workspaces/shop/feat-json/shop-frontend","outcome":"created"},{"key":"shop-backend","path":"/Users/you/.octotask/workspaces/shop/feat-json/shop-backend","outcome":"created"},{"key":"shop-agent","path":"/Users/you/.octotask/workspaces/shop/feat-json/shop-agent","outcome":"created"}]}
```

## Related pages

- [Repositories](repo.md) for the base branch of each repository.
- [Syncing](sync.md) for `octok fetch` and `octok pull`.
- [Dev changes](dev.md) for what each new checkout receives.
- [Status](status.md) for the state of every workspace.
- [Troubleshooting](../troubleshooting.md) for the errors above.
