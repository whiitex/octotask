# Workspaces

How `octok branch`, `octok checkout` and `octok rm` create, complete and delete a workspace: one folder with a checkout of every repository, all on the same branch.

A [workspace](../concepts.md#workspace) is a folder with one checkout of each repository of the [project](../concepts.md#project), all on the same branch.
Each checkout is a `git worktree` of the [cache](../concepts.md#cache) of its repository, so a workspace is created in an instant and several can be open side by side.
`octok branch` creates the workspace of a new branch, `octok checkout` the one of a branch that already exists, and `octok rm` deletes a workspace.
To build a workspace for pull requests, see [Pull request review](pr-review.md).

## Create the workspace of a new branch

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

`octok branch <branch>` fetches every repository, creates the branch in each one from its up-to-date base branch, and checks it out in a new folder `~/.octotask/workspaces/<project>/<workspace>/<repo>`.
It prints the path of the workspace on stdout, and what it did on stderr, so `cd "$(octok branch feat/cart)"` enters the workspace.
A program cannot change the folder of the shell that started it: that is why the command prints the path and the shell does the `cd`.
With `--location`, or when a repository lives on another machine, stderr also has one line per repository there, with the path of its checkout on that machine.
The path on stdout is the folder of the workspace on this machine, and `<host>:<path>` when the workspace has no folder here.
`octok checkout -b <branch>` does the same as `octok branch <branch>`.

### What each repository starts from

- The new branch starts at the tip of the base branch of the repository as the remote has it now, not at an old local copy: the command fetches first.
  The base branch is the one of the repository, or else the one of the project: see [Repositories](repo.md#the-base-branch).
- The branch does not follow the base branch.
  It exists only on your machine until you publish it.
- A repository whose remote has no base branch makes the whole command fail with exit code 3, and nothing is created.
- A project without repositories is refused with exit code 3: add one with [`octok repo add`](repo.md).
- Every new checkout gets the [dev changes](dev.md) saved for its repository.
  The command creates nothing, names the file and exits with code 5 when a dev change no longer applies to the commit a checkout starts from: `--allow-stale-dev` creates the workspace with that file as it is in git.
- If creating one checkout fails, the checkouts and the branches that the command created are removed again.
- A branch name that git does not accept is refused with exit code 2, before anything is fetched.

### The name of a workspace

A workspace is named after its branch, with every `/` replaced by `-`: the branch `feat/cart` lives in the workspace `feat-cart`, and it is the name shown by [`octok status`](status.md).
Other commands take the branch, such as `octok rm feat/cart`, and the name is derived from it.

Two branches can give the same name: `feat/cart` and `feat-cart`.
Only one of them can have a workspace, and the second is refused, with exit code 5, whether it comes from `octok branch` or from `octok checkout`:

```console
$ octok branch feat-cart
error: workspace `feat-cart` is the one of the branch `feat/cart`, whose name gives the same folder
remove it with `octok rm feat/cart`, or use another branch name
$ octok branch feat/cart
error: workspace `feat-cart` already exists at /Users/you/.octotask/workspaces/shop/feat-cart
```

The second command shows what a rerun of `octok branch` does on a workspace that is already complete: it is refused, and nothing changes.

### Publishing the branch

The new branch exists only in your checkouts.
A plain `git push` inside a checkout publishes it and sets its upstream, with no need for `-u origin <branch>`.
The cache is configured for that, so `git pull` and `git push` work in every checkout as in any clone.
Until a branch is pushed, [`octok status`](status.md) marks it `[not on the remote]`.

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

## Branches that already exist

`octok branch` is for new branches only.
It refuses a branch that is already on the remote of some repository, so that a new name never lands on somebody else's work by mistake.
`octok checkout <branch>` is the command for a branch that someone already pushed, such as the pull request of a teammate or of an agent:

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

`octok checkout <branch>` prints the path of the workspace of the branch, creating it first when it is not there:

- A repository that has the branch on its remote gets a local branch at the same commit, which follows the remote one: `git pull`, `git push` and [`octok pull`](sync.md) work on it.
- A repository that does not have the branch gets a new branch with that name from its up-to-date base, as with `octok branch`, so the workspace is complete and [`octok up`](up.md) can start everything.
  The branch stays on your machine until you push it.
  If it is pushed to that repository later, `octok pull` makes the local branch follow it.
- A workspace that is already complete is left alone: the command only prints its path, without a fetch.
- git keeps a branch in one checkout at a time, so a branch that another workspace has checked out, such as one made by [`octok pr review`](pr-review.md#one-branch-one-workspace), is refused with exit code 5 and the folder that holds it.
- A branch that is on no remote of the project is refused with exit code 3.
  The error says to create it with `octok checkout -b <branch>`, which is the same as `octok branch <branch>`:

```console
$ octok checkout feat/nothing
error: no repo has a branch `feat/nothing` on its remote: create it with `octok checkout -b feat/nothing`
```

`octok branch` and `octok checkout` print the same messages on stderr: a `created workspace` or `completed workspace` line, then the repositories by what they got.
The lines are `branch from the remote` for a branch that the remote had, `new branch from the base` for a new branch and `branch restored from the cache` for a branch whose checkout was lost.
When every checkout is new and made from the base, only the first line is printed.
Nothing is printed when the workspace was complete.

## Complete a workspace

Run the command again on a workspace that lacks some checkouts, and it adds only those.
This happens in three cases:

- A repository was registered after the workspace was created.
- The creation was interrupted.
- The folder of a checkout was deleted by hand: the checkout is restored with its commits, because the branch is still in the cache.

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

The second `octok checkout` prints the path and nothing else, because the workspace is complete.
Here `shop-agent` was registered after `feat-cart` was created, as in the example of the previous section.
The other checkouts are not touched.
`octok checkout <branch>` always completes.
`octok branch <branch>` completes too, as long as none of the repositories it adds has the branch on its remote.
A complete workspace is refused by `octok branch` as shown above, and printed by `octok checkout`.

## Delete a workspace

`octok rm <branch>` deletes the workspace of a branch: its checkouts, its folder and its local branches.
It refuses when that would lose work that exists nowhere else, and lists what it found:

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

Without `--force`, the command checks every repository and refuses, with exit code 4, when it finds:

- uncommitted changes in a checkout, including files that git does not track;
- commits on the branch that are on no branch of the remote, as of the last fetch or push;
- commits on a detached HEAD that are on no branch of the remote;
- a checkout that cannot be read;
- files or folders in the workspace that are not the checkout of a repository.

Nothing is deleted when anything is found.
`--force` deletes the workspace anyway and discards that work.
Files that git ignores, such as `.env`, are not checked: `octok rm` deletes them without a warning, with or without `--force`.
So are the saved [dev changes](dev.md) in the checkouts: they are saved elsewhere, so the next workspace gets them again.

`octok rm` deletes the folder, then the local branches:

- It deletes the local branch of the workspace in each repository.
  It also deletes the branch that the checkout had, when that differs, and the branches saved for a [pull request review](pr-review.md).
- It never deletes a branch that another workspace has checked out.
- It never deletes the branch of a remote: the branches on GitHub stay.
- It never deletes the base branch of a repository, the cache or the repository itself.
- It forgets the settings that the workspace had of its own in the project file: its location and its run commands.
  See [Locations and SSH](locations.md) and [Running](up.md).

A workspace whose folder was deleted by hand can be removed as well: `octok rm` cleans what it left in the caches.
A branch without a workspace is refused with exit code 3.

```console
$ octok rm feat/search
error: no workspace for `feat/search`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch
```

## Options

Both `octok branch` and `octok checkout` take these:

- `--location <location>`: the whole workspace lives on that machine, whatever the repositories and the project say.
  See [Locations and SSH](locations.md).
- `--allow-stale-dev`: create the workspace even if some dev changes no longer apply.
  See [Dev changes](dev.md).

With `--json`, a command prints one document with the project, the workspace, its path and, for each repository, the path of its checkout and its outcome.
See [JSON output](../agents/json-output.md).

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
