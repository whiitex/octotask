# A feature across several repositories

Build one feature that touches a frontend and a backend from start to finish: create the workspace, commit in two repositories, publish, keep up with your teammates and the base branch, add a third repository and clean up after the merge.

## How to do it

1. Register the project and its repositories once, each with a short alias: `octok project init shop`, then `octok repo add <url> --alias frontend`.
2. Create the [workspace](../concepts.md#workspace) of the new branch and enter it: `cd "$(octok branch feat/cart)"`.
   Every repository gets the branch from its up-to-date base, in the folder `~/.octotask/workspaces/shop/feat-cart`.
3. Work and commit in each checkout with plain `git`, and see where each one stands with `octok status`.
   A checkout you leave alone stays on the branch with no commit, and costs nothing.
4. Publish each repository with `git push` in its checkout, which also sets the upstream, and open each pull request with `gh pr create`.
   `octok` never pushes.
5. Bring in the commits of your teammates with `octok pull`, which only fast-forwards: it never merges, rebases or resets.
6. Bring in the base branch yourself, in each checkout that needs it: `git merge origin/staging`, or `git rebase origin/staging` and `git push --force-with-lease`.
7. Add a repository later with `octok repo add`, then `octok checkout feat/cart`.
8. After the merge, check that the work is in the base branch, then remove the workspace with `octok rm feat/cart`, or `octok rm --force feat/cart` after a squash merge.

Once the branch is on a remote, `octok branch feat/cart` is refused: get the workspace back with `octok checkout feat/cart`.
The commands are described in [Workspaces](../commands/workspaces.md), [Syncing](../commands/sync.md) and [Status](../commands/status.md), and every refusal in [Troubleshooting](../troubleshooting.md).

## Example: register the project

The project `shop` has the repositories `shop-frontend` and `shop-backend`, whose base branch is `staging`:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
```

## Example: create the workspace

`octok branch` prints the path of the workspace on stdout, and the rest on stderr, so `cd` enters it:

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ ls
shop-backend
shop-frontend
```

## Example: commit in two repositories

`octok status` marks `[not on the remote]` a branch that was not on the remote at the last fetch:

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

## Example: publish the branches

A plain `git push` creates the branch on the remote and tracks it, with no `-u origin <branch>`:

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

## Example: pull the commit of a teammate

A teammate pushes to `feat/cart` of the backend, and `octok pull` fast-forwards that checkout:

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

A checkout that has diverged is skipped and reported, and outside the workspace `octok pull feat/cart` names it.

## Example: merge the base branch

`octok pull` fetches the new commit of `staging`, but only `git merge` brings it into the branch:

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

## Example: add a repository later

`octok checkout` adds the checkouts the workspace lacks and leaves the others alone:

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

The agent has no `feat/cart` on its remote, so it gets a new branch from its base, which `octok pull` follows once it is pushed.
The same command restores a checkout whose folder you deleted by hand, with its commits.

## Example: remove the workspace after a squash merge

A squash merge puts a new commit on the base branch and can delete the remote branch, so your commits look unpushed and `octok rm` refuses:

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

Check that the work is in `git log origin/staging` of each checkout, then force the removal, which discards what the workspace holds:

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

`octok rm` also deletes the local branches, and files that git ignores, such as `.env`, go without a warning: keep them as [dev changes](../commands/dev.md).

## Related pages

- [Workspaces](../commands/workspaces.md), for `octok branch`, `octok checkout` and `octok rm`.
- [Syncing](../commands/sync.md), for `octok pull`.
- [Repositories](../commands/repo.md), for `octok repo add`.
- [Troubleshooting](../troubleshooting.md).
