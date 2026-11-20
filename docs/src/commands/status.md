# Status

What `octok status` prints for a project: its repositories, its workspaces, the checkout of each repository in each workspace, and the problems it finds.

`octok status` shows where everything stands in the default project, or in the one named with `-p`.
It answers three questions: what each repository's base branch looks like, what each workspace has checked out, and what is wrong.
It reads the project file, the caches and the checkouts on disk, and it never contacts a git remote.

## The usual output

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

## What each part shows

### `project:`

The name of the project.

### `repos:`

One line per repository, in the order of the project:

```text
  <key> (<alias>)  <base branch>  <last commit of the base branch>
```

- The label is the key, followed by the alias in parentheses when there is one.
- The base branch is the one in effect: the repository's own, else the project's.
- The commit is the tip of that base branch in the cache, as of the last fetch: its abbreviated hash, its subject and its age.
  It is not asked to the remote.
- A repository on another machine ends with its location in brackets, such as `[api-dev-1:/home/ubuntu/.octotask]`.
  A repository on this machine shows nothing.

Instead of the commit, a line can say:

| Text | Meaning |
|---|---|
| ``branch `<base>` is not on the remote`` | The repository is cloned, but its base branch was not on the remote at the last fetch. New branches cannot start from it: fix it with [`octok repo set <repo> --base`](repo.md) or [`octok project set --base`](project.md). |
| ``not fetched (run `octok fetch`)`` | There is no clone of the repository yet. `octok fetch` makes it. |

A project without repositories shows ``repos: none (add one with `octok repo add <url>`)``.

### `workspaces:`

One group for each workspace the project has, in alphabetical order of their names.
The first line has the name of the workspace and its folder, which is the one on this machine when the workspace has one there.
Below it, one row for every repository of the project, in the order of the project, so a repository without a checkout stands out:

```text
    <key> (<alias>)  <branch>  <commit of the checkout>  [<marker>]
```

- The branch is the one the checkout is on, `(detached)` on a detached HEAD.
- The commit is the one checked out: hash, subject and age.
- `[not on the remote]` follows the commit of a checkout whose branch was not on the remote at the last fetch.
  It is what a new branch shows until a `git push`, and what a branch shows after it was deleted on the remote and a fetch dropped it.
- `[<host>:<path>]` follows the commit of a checkout that lives on another machine, and tells where.
- `` no worktree: add it with `octok checkout <branch>` `` replaces a repository that has no checkout in the workspace, for example one registered after the workspace was created.
  See [Workspaces](workspaces.md#complete-a-workspace).
- `cannot be read: <reason>` replaces a checkout whose branch and commit could not be read, with the last line of the error of the tool.
  It does not hide the other repositories, and the command still exits with code 0.

A project without workspaces shows ``workspaces: none (create one with `octok branch <name>`)``.

### `orphan caches`

After the workspaces, a section lists the clones in the cache folder that no repository of the project owns, one path per line, on every machine the project uses, as `<host>:<path>` on another machine:

```text
orphan caches (not in the project file):
  /Users/you/.octotask/cache/shop/old.git
```

They are left behind when a repository key is renamed by hand in the project file.
`octok status` only reports them: nothing deletes them, and you can delete the folder if you do not need it.

## When something is off

This session breaks a project in several ways at once, to show how `octok status` reports each of them:

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

Reading the output from the top:

- `shop-frontend` and `shop-agent` are cloned, but `develop` is not on their remotes.
- `shop-backend` has no clone anymore, so it was never fetched as far as `octok` can tell: `octok fetch` clones it again.
- The checkout of `shop-frontend` is on a detached HEAD, so it shows `(detached)` and no `[not on the remote]` marker.
- The checkout of `shop-backend` cannot be read, because its commits lived in the clone that is gone.
  After `cannot be read:` comes the error of `git` as it is, shown here as `[..]` because its text changes with the version of `git`.
  The other rows are not affected.
- `shop-agent` has no checkout in `feat-cart`, because it was registered after it.
  The hint says `<branch>` here only because no checkout of the workspace has a branch left to read: it is the name of the branch otherwise.
- The clone `old.git` belongs to no repository of the project.

## Repositories on another machine

A repository whose [location](../concepts.md#location) is another machine ends its `repos:` line with that location.
In a workspace, the checkouts that live there end with their path on that machine:

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

## What it reads, and what it never does

`octok status` reads:

- the project file;
- the clone of each repository, for the remote branches as they were at the last fetch and for the commit of the base branch;
- the checkout of each repository in each workspace, for its branch and its last commit.

It does not run `git fetch`, `git ls-remote` or anything else that contacts a git remote, and it changes nothing on disk.
That is why the markers say "at the last fetch": a branch that someone pushed a minute ago is not in the output until `octok fetch` or [`octok pull`](sync.md).
The one network use is `ssh`, to read the clones and the checkouts that live on another machine: see [Locations and SSH](locations.md).
Without a terminal, such a connection never asks a question: it fails instead.
A machine that cannot be reached fails the whole command with the error of `ssh` and exit code 7, unlike a single checkout that cannot be read.

The output is meant for people.
With `--json`, `octok status` prints one document with the same information, and with more detail for each repository and checkout: see [JSON output](../agents/json-output.md).
When the reader of the output goes away, as in `octok status | head -1`, the command ends quietly.

## Related pages

- [Workspaces](workspaces.md) for creating and completing the workspaces it lists.
- [Syncing](sync.md) for `octok fetch`, which refreshes what it shows.
- [Repositories](repo.md) and [Projects](project.md) for the base branch it reports.
