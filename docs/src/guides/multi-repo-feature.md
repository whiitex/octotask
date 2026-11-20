# A feature across several repositories

Build one feature that touches a frontend and a backend from start to finish: create the workspace, commit in two repositories, publish, keep up with your teammates and the base branch, add a third repository and clean up after the merge.

Every command of this page is run against the real `octok` when the documentation is tested, so the output you see is what you get.
The names are the ones of the examples everywhere in this documentation: the project `shop`, with the repositories `shop-frontend` and `shop-backend`, whose base branch is `staging`.
A [workspace](../concepts.md#workspace) is one folder with a checkout of every repository, all on the same branch.

## Start from a project

Register the project and its two repositories once.
Each repository gets an alias, a short name that every command accepts in place of the key.
See [Projects](../commands/project.md) and [Repositories](../commands/repo.md) for every option.

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
```

## Create the workspace

`octok branch` fetches every repository, creates the branch from the up-to-date base branch of each one and checks it out in a new workspace.
It prints the path of the workspace on stdout, so a `cd` around it enters the folder.
What it says about the creation goes to stderr.

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ ls
shop-backend
shop-frontend
```

The workspace is named after its branch, with every `/` turned into `-`.
Its folder is `~/.octotask/workspaces/shop/feat-cart`, and each repository is a normal git working tree inside it.

## Work and commit in two repositories

Work in each checkout with plain `git`.
The branch is new, so it exists only on your machine until you push it.

```console
$ cd shop-backend
$ echo 'def cart(): ...' > cart.py
$ git add cart.py
$ git commit --quiet -m "Add the cart endpoint"
$ cd ../shop-frontend
$ echo 'export const cart = []' > src/cart.js
$ git add src/cart.js
$ git commit --quiet -m "Add the cart store"
$ cd ..
```

You do not have to touch every repository.
A checkout you leave alone stays on its branch with no commit, and it costs nothing.

`octok status` shows where each checkout stands.
A checkout whose branch was not on the remote at the last fetch is marked `[not on the remote]`.

```console
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  feat/cart  30359cf  Add the cart store (10 minutes ago)  [not on the remote]
    shop-backend (backend)    feat/cart  966f9ed  Add the cart endpoint (10 minutes ago)  [not on the remote]
```

## Publish the branches

`octok` never pushes.
Publish each repository with a plain `git push` inside its checkout.
It creates the branch on the remote and sets its upstream, so you do not need `-u origin <branch>`.
Open the pull request of each repository with `gh pr create`, once per repository.

```console
$ git -C shop-backend push
To git@github.com:acme/shop-backend.git
 * [new branch]      feat/cart -> feat/cart
branch 'feat/cart' set up to track 'origin/feat/cart'.
$ git -C shop-frontend push
To git@github.com:acme/shop-frontend.git
 * [new branch]      feat/cart -> feat/cart
branch 'feat/cart' set up to track 'origin/feat/cart'.
```

Once a branch is on a remote, `octok branch feat/cart` is refused for a new workspace, so that a new name never lands on someone else's work: use `octok checkout feat/cart` to get the workspace back.

## Keep up with your teammates

`octok pull` fetches every repository and fast-forwards each checkout to its remote branch.
It prints one line per repository.
Here a teammate pushed a commit to `feat/cart` of the backend:

```console
$ git clone --quiet git@github.com:acme/shop-backend.git "$HOME/mate"
$ git -C "$HOME/mate" switch --quiet feat/cart
$ echo '# reviewed' >> "$HOME/mate/cart.py"
$ git -C "$HOME/mate" commit --quiet -am "Review the cart endpoint"
$ git -C "$HOME/mate" push --quiet
$ octok pull
shop-frontend (frontend)  up to date
shop-backend (backend)    updated
```

`octok pull` only fast-forwards: it never merges, rebases or resets.
A checkout that has diverged from its remote branch is skipped and reported, and you bring it in line with `git merge` or `git rebase` in that checkout.
The same happens to a checkout whose branch is not on the remote yet, or that is on a detached HEAD.
Run `octok pull` from inside a workspace to use that workspace, or give it the branch: `octok pull feat/cart`.
It exits with code 6 when any repository failed, and it goes on with the others.
See [Syncing](../commands/sync.md).

## Keep up with the base branch

The base branch of a repository moves while you work.
`octok pull` fetches it, but it does not merge it into your branch: you do that in each checkout with `git`.
Here the base branch of the backend got a commit:

```console
$ git -C "$HOME/mate" switch --quiet staging
$ echo '# base change' >> "$HOME/mate/orders.py"
$ git -C "$HOME/mate" commit --quiet -am "Tidy the orders"
$ git -C "$HOME/mate" push --quiet
$ octok pull
shop-frontend (frontend)  up to date
shop-backend (backend)    up to date
$ git -C shop-backend log --oneline HEAD..origin/staging
[..] Tidy the orders
$ git -C shop-backend merge --quiet --no-edit origin/staging
$ git -C shop-backend push --quiet
$ octok pull
shop-frontend (frontend)  up to date
shop-backend (backend)    up to date
```

Use `git rebase origin/staging` instead of the merge when your team rebases, and push with `--force-with-lease`.
A repository in which `HEAD..origin/staging` is empty has nothing to bring in.

## Add a repository later

A feature sometimes turns out to need another repository.
Register it, then run `octok checkout` on the branch: it adds the checkouts the workspace lacks and leaves the others alone.

```console
$ octok repo add git@github.com:acme/shop-agent.git --alias agent
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
$ octok checkout feat/cart
completed workspace `feat-cart` in shop
  new branch from the base: shop-agent
/Users/you/.octotask/workspaces/shop/feat-cart
$ ls
shop-agent
shop-backend
shop-frontend
```

The agent has no `feat/cart` on its remote, so it gets a new branch with that name from its up-to-date base.
When the branch is pushed to that repository later, `octok pull` makes the local branch follow it.
`octok branch feat/cart` does the same only while the branch is on no remote.
The same command also restores a checkout whose folder you deleted by hand, with its commits.

## Clean up after the merge

When the pull requests are merged, remove the workspace with `octok rm`.
It refuses to delete work that exists nowhere else: uncommitted changes, unpushed commits, and files that do not belong to a checkout.

A squash merge makes this visible.
GitHub puts a new commit on the base branch and, when the repository deletes merged branches, deletes the branch of the pull request: your own commits then look unpushed.
Here the remote branches are gone, as after the merge of both pull requests:

```console
$ git -C "$HOME/mate" push --quiet origin --delete feat/cart
$ git -C shop-frontend push --quiet origin --delete feat/cart
$ octok pull
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    skipped: branch is not on the remote yet
shop-agent (agent)        skipped: branch is not on the remote yet
$ octok rm feat/cart
error: workspace `feat-cart` has work that would be lost:
  shop-frontend: 1 unpushed commit on `feat/cart`
  shop-backend: 3 unpushed commits on `feat/cart`
remove it anyway with `octok rm --force <branch>`
```

`octok` cannot tell a squash-merged commit from an unpushed one, so it reports both.
Check that the work is in the base branch of each repository, for example with `git log origin/staging` in its checkout, then remove the workspace with `--force`:

```console
$ octok rm --force feat/cart
removed workspace `feat-cart` from shop
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  5578dd8  Tidy the orders (10 minutes ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)

workspaces: none (create one with `octok branch <name>`)
```

`--force` discards what the workspace holds, so use it only after that check.
`octok rm` also deletes the local branches of the workspace.
Files that git ignores, such as `.env`, go without a warning: if you keep your local setup as [dev changes](../commands/dev.md), they are saved elsewhere.

## When a step is refused

Each refusal says what to do.
The ones this recipe can meet:

| You see | Because | Do this |
|---|---|---|
| `branch ... already exists on the remote of: ...` | `octok branch` never lands on a branch that exists | `octok checkout <branch>`, or choose another name |
| `workspace ... already exists at ...` | The workspace of that branch is there | `cd "$(octok checkout <branch>)"` |
| `workspace ... is the one of the branch ..., whose name gives the same folder` | `feat/cart` and `feat-cart` are both `feat-cart` | Remove the other workspace, or use another name |
| `workspace ... has work that would be lost` | `octok rm` found uncommitted or unpushed work | Commit and push, or check the merge and use `--force` |
| `skipped: branch is not on the remote yet` | `octok pull` found no remote branch | Push it with `git push` |

All of them, with their exit codes, are in [Troubleshooting](../troubleshooting.md).
The commands used here are described in [Workspaces](../commands/workspaces.md), [Syncing](../commands/sync.md) and [Status](../commands/status.md).
