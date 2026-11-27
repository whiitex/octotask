# Pull request review

How `octok pr review` builds one workspace with each repository on a pull request or a branch you choose, and what it needs from `gh`.

## How to use it

1. For `--pr` and the picker, install the GitHub CLI `gh` and sign in with `gh auth login`.
2. Name a pull request or a branch per repository, and enter the workspace: `cd "$(octok pr review --pr backend=12 --branch agent=fix/prompt)"`.
   At a terminal, `octok pr review` alone lets you pick from the open pull requests.
3. Run it with [`octok up --branch pr-backend-12`](up.md) and update it with [`octok pull pr-backend-12`](sync.md), or run both from inside the workspace.
4. Remove it with [`octok rm pr-backend-12`](workspaces.md#example-delete-a-workspace).

## Options

| Option | What it does |
|---|---|
| `--pr <repo>=<number>` | Takes the branch of that pull request, open, closed or merged. `12` and `#12` both work. Repeatable. |
| `--branch <repo>=<branch>` | Takes a branch of the remote of `<repo>`, without `gh`. Repeatable. Needed for a branch named only with digits. |
| `--name <name>` | Names the workspace and the new branch of the other repositories. It must be a valid branch name. |
| `--location <location>` | Puts the whole workspace on one machine: see [Locations and SSH](locations.md). |
| `--allow-stale-dev` | Creates the workspace even when saved [dev changes](dev.md) no longer apply. Without it, nothing is created and the error names the file. |

`<repo>` is the key or the alias of a repository, given once.
See the [command reference](../reference/commands.md#octok-pr-review) for the generated list.

## Rules

- **Named repositories** get a local branch at the remote branch, fetched just before, which follows it for `git pull`, `git push` and `octok pull`.
- **Other repositories** get a new branch from their up-to-date [base branch](../concepts.md#base-branch), named after the workspace, local until you push it.
  If the remote already has a branch with that name, they get that one.
- **Output:** `` <repo>: pull request #12 is the branch `feat/login` `` goes to stderr before anything is created, and the path to stdout, so `cd "$(...)"` works.
- **Default name:** `pr-<repo>-<number>` after the first pull request in project order, with the alias when there is one.
  With only `--branch`, the first branch names both: `--branch agent=fix/prompt` makes the workspace `fix-prompt` and the branch `fix/prompt` in the other repositories.
- **The picker** lists the 15 newest open pull requests of each repository, titles cut to 72 characters.
  Type a number to take a pull request, a name to take a branch, or Enter to stay on the base.
  An answer that cannot be reviewed is asked again, and when `gh` cannot list, `octok` says why and still asks.
  A repository without any prints `No open pull requests of <repo>.`
  Choosing nothing anywhere stops with `nothing to review`.
- **No picker** with `--json` or without a terminal: give at least one `--pr` or `--branch`.
  It also does not start when the command cannot succeed, such as with no repositories or an invalid `--name`.
- **Remembered:** the project file keeps what the workspace reviews until `octok rm`.
  The same command again prints the path and changes nothing, and [`octok checkout <name>`](workspaces.md) completes the workspace on the same branches.
- **Forks** are refused, because their branch is not in the repository itself, and the picker marks them `from a fork`.
- **One branch, one workspace:** a branch that another workspace has, such as `feat/login` after `octok checkout feat/login`, is refused with the folder that holds it.
- **`gh`** runs on this machine, also for a repository on [another machine](locations.md), and fails with its own message for a repository not on GitHub or when not signed in.
  Nothing else in `octok` needs it.
- **Nothing left behind:** every refusal comes before anything is created, and `octok rm` also deletes the local branches only this workspace used.

| Exit code | When |
|---|---|
| 0 | The workspace was created, or already reviews the same choice. |
| 1 | `gh` failed, or ``pull request #<number> of `<repo>` comes from a fork: octok can only review a branch of the repo itself``. |
| 2 | Nothing to review, a repository given twice, or an invalid value. |
| 3 | A repository not registered, or a branch not on its remote. |
| 5 | The workspace exists with another choice, or its name belongs to a workspace of `octok branch` or `octok checkout`. |
| 7 | `` `gh` not found: install it or add it to PATH ``. |

## Example: review a pull request and a branch

Pull request #12 of the backend is the branch `feat/login`, and the agent has a branch `fix/prompt` with no pull request:

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
$ cd "$(octok pr review --pr backend=12 --branch agent=fix/prompt)"
shop-backend (backend): pull request #12 is the branch `feat/login`
created workspace `pr-backend-12` in shop
  branch from the remote: shop-backend, shop-agent
  new branch from the base: shop-frontend
$ ls
shop-agent
shop-backend
shop-frontend
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)

workspaces:
  pr-backend-12  /Users/you/.octotask/workspaces/shop/pr-backend-12
    shop-frontend (frontend)  pr-backend-12  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/login     ec75932  Add login endpoint (3 hours ago)
    shop-agent (agent)        fix/prompt     1967eea  Shorten the prompt (6 hours ago)
```

## Example: the picker

At a terminal, without `--pr` or `--branch`, each repository gets a question:

```text
Open pull requests of shop-backend (backend):
  #12  Add login endpoint  (feat/login, alice)
  #13  Fix typo  (patch-1, alice, from a fork)
Pull request or branch of shop-backend (backend) [none, base `staging`]:
```

## Example: run it again and complete it

The same choice reuses the workspace, another one is refused, and `octok checkout` restores a deleted checkout on its branch:

```console
$ octok pr review --pr backend=12 --branch agent=fix/prompt
shop-backend (backend): pull request #12 is the branch `feat/login`
/Users/you/.octotask/workspaces/shop/pr-backend-12
$ octok pr review --pr backend=12 --branch frontend=feat/login --name pr-backend-12
shop-backend (backend): pull request #12 is the branch `feat/login`
error: workspace `pr-backend-12` already reviews other branches: shop-backend=feat/login, shop-agent=fix/prompt
remove it with `octok rm pr-backend-12`, or choose another name with `--name`
$ rm -rf "$OCTOTASK_HOME/workspaces/shop/pr-backend-12/shop-agent"
$ octok checkout pr-backend-12
completed workspace `pr-backend-12` in shop
  branch restored from the cache: shop-agent
/Users/you/.octotask/workspaces/shop/pr-backend-12
$ git -C "$OCTOTASK_HOME/workspaces/shop/pr-backend-12/shop-agent" branch --show-current
fix/prompt
```

## Example: what is refused

A branch already in another workspace:

```text
error: branch `feat/login` of `shop-backend` is already checked out in /Users/you/.octotask/workspaces/shop/feat-login/shop-backend
a branch lives in one workspace at a time: use that workspace, or remove it first
```

The other refusals:

```console
$ octok pr review
error: nothing to review: choose a pull request or a branch for at least one repo, with `--pr <repo>=<number>` or `--branch <repo>=<branch>`, or by answering `octok pr review` at a terminal
$ octok pr review --pr backend=12 --branch backend=feat/login
error: repo `shop-backend` is given more than once: choose one pull request or one branch for it
$ octok pr review --pr backend=nope
error: invalid value 'backend=nope' for '--pr <REPO=NUMBER>': expected `<repo>=<number>`, e.g. `backend=12`

For more information, try '--help'.
$ octok pr review --branch agent=feat/nope
error: repo `shop-agent` has no branch `feat/nope` on its remote
$ octok pr review --pr nothing=1
error: repo `nothing` is not registered
$ octok pr review --pr backend=99
error: `gh pr view 99 --repo git@github.com:acme/shop-backend.git --json number,title,headRefName,author,isCrossRepository` failed in /Users/you/.octotask (exit status: 1)
GraphQL: Could not resolve to a PullRequest with the number of 99. (repository.pullRequest)
```

## Related pages

- [Workspaces](workspaces.md), for `octok checkout` and `octok rm`.
- [Syncing](sync.md), for `octok pull`.
- [Running](up.md), to run the workspace.
- [Locations and SSH](locations.md), for `--location`.
- [Troubleshooting](../troubleshooting.md).
