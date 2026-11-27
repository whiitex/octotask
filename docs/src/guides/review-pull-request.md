# Reviewing a pull request

Open a teammate's change in a workspace to read it, run it and review it, whether their branches share one name or their pull requests are named one by one.

## How to do it

1. Open the change in a [workspace](../concepts.md#workspace) of its own, next to yours.
   Every repository without the change gets a new branch from its up-to-date base, so you can run the whole system.
   - When the branches share one name, run `cd "$(octok checkout feat/login)"`.
   - When the pull requests are named one by one, sign in with `gh auth login`, then run `cd "$(octok pr review --pr backend=12 --branch agent=fix/prompt)"`.
2. Read the change with plain `git` in each checkout, for example `git log origin/staging..HEAD`.
3. Start it with `octok up --branch pr-backend-12`, once each repository has a [run command](../commands/up.md).
   Save your local settings once as [dev changes](../commands/dev.md), and every review workspace gets them.
4. When your teammate pushes again, bring it in with `octok pull pr-backend-12`.
5. When the review is done, remove the workspace with `octok rm pr-backend-12`: the branches of your teammate stay on the remote.

A branch lives in one workspace at a time, so remove a workspace before you review its branch in another one.
The options, the names of the workspaces and the questions at a terminal are in [Pull request review](../commands/pr-review.md) and [Workspaces](../commands/workspaces.md), and every refusal in [Troubleshooting](../troubleshooting.md).

## Example: register the project

The project `shop` has the repositories `shop-frontend`, `shop-backend` and `shop-agent`:

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
```

## Example: branches that share a name

`feat/login` is on the frontend and the backend, so they follow it, and the agent gets a new local branch from its base:

```console
$ cd "$(octok checkout feat/login)"
created workspace `feat-login` in shop
  branch from the remote: shop-frontend, shop-backend
  new branch from the base: shop-agent
$ ls
shop-agent
shop-backend
shop-frontend
```

```console
$ git -C shop-backend log --oneline origin/staging..HEAD
ec75932 Add login endpoint
$ git -C shop-backend diff --stat origin/staging...HEAD
 auth.py | [..]
 1 file changed, [..]
```

`octok checkout` refuses a branch that is on no remote, with a hint to create it with `octok branch`.

## Example: pull requests named one by one

The backend has pull request #12, the agent the branch `fix/prompt`, and the frontend nothing.
Pull request #12 is `feat/login`, still checked out in the workspace above, so the review is refused until that workspace is removed:

```console
$ cd
$ octok pr review --pr backend=12 --branch agent=fix/prompt
shop-backend (backend): pull request #12 is the branch `feat/login`
error: branch `feat/login` of `shop-backend` is already checked out in /Users/you/.octotask/workspaces/shop/feat-login/shop-backend
a branch lives in one workspace at a time: use that workspace, or remove it first
$ octok rm feat/login
removed workspace `feat-login` from shop
```

```console
$ cd "$(octok pr review --pr backend=12 --branch agent=fix/prompt)"
shop-backend (backend): pull request #12 is the branch `feat/login`
created workspace `pr-backend-12` in shop
  branch from the remote: shop-backend, shop-agent
  new branch from the base: shop-frontend
$ ls
shop-agent
shop-backend
shop-frontend
```

The workspace is named `pr-<repo>-<number>` after the first pull request, and `--name` chooses another name.

## Example: they push again

`octok pull` fast-forwards the backend, and skips the frontend, whose branch exists only on your machine:

```console
$ git clone --quiet git@github.com:acme/shop-backend.git "$HOME/mate"
$ git -C "$HOME/mate" switch --quiet feat/login
$ echo '# more' >> "$HOME/mate/auth.py"
$ git -C "$HOME/mate" commit --quiet -am "Check the token"
$ git -C "$HOME/mate" push --quiet
$ octok pull pr-backend-12
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    updated
shop-agent (agent)        up to date
$ git -C shop-backend log --oneline -2
a3ff744 Check the token
ec75932 Add login endpoint
```

The workspace remembers its choices until `octok rm`: run the same `octok pr review`, or `octok checkout pr-backend-12`, to restore a missing checkout.

## Example: remove it

```console
$ octok rm pr-backend-12
removed workspace `pr-backend-12` from shop
```

## Example: what is refused

```console
$ cd
$ octok pr review
error: nothing to review: choose a pull request or a branch for at least one repo, with `--pr <repo>=<number>` or `--branch <repo>=<branch>`, or by answering `octok pr review` at a terminal
$ octok pr review --pr backend=12 --branch backend=feat/login
error: repo `shop-backend` is given more than once: choose one pull request or one branch for it
$ octok pr review --pr backend=99
error: `gh pr view 99 --repo git@github.com:acme/shop-backend.git --json number,title,headRefName,author,isCrossRepository` failed in /Users/you/.octotask (exit status: 1)
GraphQL: Could not resolve to a PullRequest with the number of 99. (repository.pullRequest)
```

## Related pages

- [Pull request review](../commands/pr-review.md), for `octok pr review`.
- [Workspaces](../commands/workspaces.md), for `octok checkout` and `octok rm`.
- [Running](../commands/up.md), for `octok up`.
- [Troubleshooting](../troubleshooting.md).
