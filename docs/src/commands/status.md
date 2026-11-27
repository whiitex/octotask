# Status

What `octok status` prints for a project: its repositories, its workspaces, the checkout of each repository in each workspace, and the problems it finds.

## How to use it

1. Refresh what the remotes have with `octok fetch` or [`octok pull`](sync.md), since `octok status` never contacts a git remote.
2. Run `octok status` for the default project, or `octok status -p <project>` for another one.
3. Fix what it reports, with the hint on each line.

## Options

| Command | What it does |
|---|---|
| `octok status` | Prints the project, its repositories, its workspaces and the orphan caches. |
| `octok status -p <project>` | Does the same for the named project. |
| `octok status --json` | Prints one document with the same information and more detail: see [JSON output](../agents/json-output.md). |

## Rules

- **Read only:** `octok status` reads the project file, the clones and the checkouts, and changes nothing on disk.
  Remote branches are as of the last fetch, so a branch pushed a minute ago is not in the output until `octok fetch`.
  The only network use is `ssh`, for the repositories on [another machine](locations.md).
- **`repos:`** has a line per repository, in project order: key, alias, base branch in effect (the repository's own, else the project's) and the tip of that branch as of the last fetch.
  A repository on another machine ends with its location, such as `[api-dev-1:/home/ubuntu/.octotask]`.
- **`workspaces:`** has a group per workspace, in alphabetical order, with its folder on this machine when it has one.
  Below it, a row per repository of the project, with the branch and commit of its checkout, so a missing checkout stands out.
- **`orphan caches`** lists the clones in the cache folder that no repository of the project owns, on every machine the project uses, as `<host>:<path>` on another machine.
  A key renamed by hand in the project file leaves one behind; `octok status` only reports it, and you can delete the folder.
- **Errors:** a checkout that cannot be read does not hide the other rows, and the command still exits with code 0.
  A machine that cannot be reached fails the whole command with the error of `ssh` and exit code 7; without a terminal, `ssh` fails instead of asking a question.
- **A closed pipe**, as in `octok status | head -1`, ends the command quietly.

What a line can show:

| Text | Where | Meaning and fix |
|---|---|---|
| ``branch `<base>` is not on the remote`` | `repos:` | The base branch was not on the remote at the last fetch, so new branches cannot start from it: fix it with [`octok repo set <repo> --base`](repo.md) or [`octok project set --base`](project.md). |
| ``not fetched (run `octok fetch`)`` | `repos:` | There is no clone yet: `octok fetch` makes it. |
| `(detached)` | checkout | The checkout is on a detached HEAD. |
| `[not on the remote]` | checkout | The branch was not on the remote at the last fetch: a new branch until its `git push`, or a branch deleted on the remote. |
| `[<host>:<path>]` | checkout | The checkout lives on another machine, at that path. |
| `` no worktree: add it with `octok checkout <branch>` `` | checkout | The repository has no checkout in the workspace, for example because it was registered later: see [Workspaces](workspaces.md#example-complete-a-workspace). The hint names the branch, or says `<branch>` when no checkout of the workspace has one to read. |
| `cannot be read: <reason>` | checkout | The branch and commit could not be read; the reason is the last line of the error of `git`. |
| `none (add one with ...)` | `repos:`, `workspaces:` | The project has no repositories, or no workspaces. |

## Example: a project with a workspace

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok status
project: shop

repos: none (add one with `octok repo add <url>`)

workspaces: none (create one with `octok branch <name>`)
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/cart  de08597  Fix order totals (5 hours ago)  [not on the remote]
```

## Example: when something is off

This session breaks the project in several ways at once: a detached checkout, a repository added after the workspace, a deleted clone, a copied clone and a base branch that is not on the remotes.
The deleted clone shows `not fetched`, and its checkout `cannot be read`, because its commits lived in that clone; `[..]` stands for the error of `git`, which changes with its version.

```console
$ cd shop-frontend
$ git switch --quiet --detach
$ cd ..
$ octok repo add git@github.com:acme/shop-agent.git --alias agent
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
$ rm -rf "$HOME/.octotask/cache/shop/shop-backend.git"
$ cp -R "$HOME/.octotask/cache/shop/shop-frontend.git" "$HOME/.octotask/cache/shop/old.git"
$ octok project set --base develop
warning: branch `develop` is not on the remote of: shop-frontend, shop-agent
base of `shop` is now `develop`
$ octok status
project: shop

repos:
  shop-frontend (frontend)  develop  branch `develop` is not on the remote
  shop-backend (backend)    develop  not fetched (run `octok fetch`)
  shop-agent (agent)        develop  branch `develop` is not on the remote

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  (detached)  1ec5189  Add cart page (2 days ago)
    shop-backend (backend)    cannot be read: fatal: [..]
    shop-agent (agent)        no worktree: add it with `octok checkout <branch>`

orphan caches (not in the project file):
  /Users/you/.octotask/cache/shop/old.git
```

## Example: a repository on another machine

```console
$ octok project init remote
created project `remote` (base: staging) at /Users/you/.octotask/projects/remote.yaml
`remote` is now the default project
$ octok repo add git@github.com:acme/shop-agent.git --location api-dev-1
added `shop-agent` to remote: git@github.com:acme/shop-agent.git
cloned at api-dev-1:/home/ubuntu/.octotask
$ octok repo add git@github.com:acme/shop-frontend.git
added `shop-frontend` to remote: git@github.com:acme/shop-frontend.git
$ octok branch feat/search
created workspace `feat-search` in remote
  shop-agent: api-dev-1:/home/ubuntu/.octotask/workspaces/remote/feat-search/shop-agent
/Users/you/.octotask/workspaces/remote/feat-search
$ octok status
project: remote

repos:
  shop-agent     staging  a1ea388  Tune the prompt (3 days ago)  [api-dev-1:/home/ubuntu/.octotask]
  shop-frontend  staging  1ec5189  Add cart page (2 days ago)

workspaces:
  feat-search  /Users/you/.octotask/workspaces/remote/feat-search
    shop-agent     feat/search  a1ea388  Tune the prompt (3 days ago)  [not on the remote]  [api-dev-1:/home/ubuntu/.octotask/workspaces/remote/feat-search/shop-agent]
    shop-frontend  feat/search  1ec5189  Add cart page (2 days ago)  [not on the remote]
```

## Related pages

- [Workspaces](workspaces.md), for creating and completing the workspaces it lists.
- [Syncing](sync.md), for `octok fetch`, which refreshes what it shows.
- [Repositories](repo.md) and [Projects](project.md), for the base branch it reports.
- [Locations and SSH](locations.md), for repositories on another machine.
