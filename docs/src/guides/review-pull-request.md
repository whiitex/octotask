# Reviewing a pull request

Open a teammate's change in a workspace to read it, run it and review it, whether their branches share one name or their pull requests are named one by one.

A teammate asks you to review a change that spans several repositories.
You want every repository on their code at the same time, so that you can run the whole system, without disturbing the feature you are working on.
`octok` gives you a [workspace](../concepts.md#workspace) for that, next to your own.
The examples use the project `shop` with the repositories `shop-frontend`, `shop-backend` and `shop-agent`, and every command is run against the real `octok` when the documentation is tested.

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

## When the branches share a name

If your teammate used the same branch name everywhere, `octok checkout` builds the workspace.
Here `feat/login` is on the frontend and on the backend, and not on the agent:

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

- Each repository that has the branch on its remote gets a local branch at the same commit, which follows the remote one.
- Each other repository gets a new branch with that name from its up-to-date base, so the workspace is complete and you can start everything.
  That branch stays on your machine until you push it.
- The workspace is named after the branch, with `/` turned into `-`: `feat-login`.

Read the change with plain `git` in each checkout:

```console
$ git -C shop-backend log --oneline origin/staging..HEAD
ec75932 Add login endpoint
$ git -C shop-backend diff --stat origin/staging...HEAD
 auth.py | [..]
 1 file changed, [..]
```

`octok checkout` is for a branch that someone already pushed.
A branch that is on no remote is refused with a hint to create it with `octok branch`.
See [Workspaces](../commands/workspaces.md).

## When the pull requests are named one by one

A change often spans pull requests with different branch names: one in the backend, another in the agent, nothing in the frontend.
`octok pr review` takes each repository with the pull request or the branch you choose, and puts every other repository on its base.
It reads a pull request with `gh`, which must be signed in with `gh auth login`.

A branch lives in one workspace at a time, so remove the first workspace before reviewing the same branch again.
Without that, the command says where the branch is:

```console
$ cd
$ octok pr review --pr backend=12 --branch agent=fix/prompt
shop-backend (backend): pull request #12 is the branch `feat/login`
error: branch `feat/login` of `shop-backend` is already checked out in /Users/you/.octotask/workspaces/shop/feat-login/shop-backend
a branch lives in one workspace at a time: use that workspace, or remove it first
$ octok rm feat/login
removed workspace `feat-login` from shop
```

The workspace had no work of yours, so `octok rm` did not object.
Now the review:

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

- `--pr <repo>=<number>` takes the branch of that pull request, whatever its state.
  `<repo>` is the key or the alias of a repository.
- `--branch <repo>=<branch>` takes that branch of the repository.
  A repository can be given once, with one of the two.
- A repository named by neither gets a new branch from its base, named after the workspace.
  If its remote already has a branch with that name, it gets that branch instead.
- The workspace is named `pr-<repo>-<number>` after the first pull request, in the order of the repositories of the project, with the alias of the repository when it has one.
  With only `--branch`, it is named after the first of those branches.
  `--name` chooses another name, and that name is what the other commands take: `octok pull pr-backend-12`, `octok rm pr-backend-12`.
- At a terminal, without `--pr` and `--branch`, `octok` lists the open pull requests of each repository and asks what to take for each one.
  Without a terminal, at least one of them is needed.

See [Pull request review](../commands/pr-review.md) for the whole behavior.

## Run it

Start the repositories of the workspace together with `octok up`, which needs a `run` command for each repository you want started:

```sh
octok up --branch pr-backend-12
```

`octok up` shows the output of every repository with its name in front, and stops them all at Ctrl-C.
It cannot run in the checked examples of this page: see [Running](../commands/up.md) for how it works and what it prints.
To review with your own settings, save them once as [dev changes](../commands/dev.md): every workspace that `octok pr review` creates gets them.

## When they push again

The checkouts follow the remote branches.
After your teammate pushes a fix, `octok pull` brings it in, by the name of the workspace:

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

The frontend is skipped because its branch, made from the base, exists only on your machine: that is expected in a review.
`octok pull` only fast-forwards, and a checkout where you committed on top of the branch is reported when it diverges.

Run the same `octok pr review` command again, or `octok checkout pr-backend-12`, to add what the workspace lacks: the choices of a workspace are remembered until `octok rm`, and the same choices only restore missing checkouts, each on the branch it had.
Other choices for a workspace that exists are refused, with a hint to remove it or to give `--name`.

## Remove it

When the review is done, remove the workspace.
The branches of your teammate stay on the remote.

```console
$ octok rm pr-backend-12
removed workspace `pr-backend-12` from shop
```

## When a step is refused

| You see | Because | Do this |
|---|---|---|
| `nothing to review: choose a pull request or a branch ...` | No `--pr` or `--branch`, and no terminal to ask at | Give at least one |
| `repo ... is given more than once` | `--pr` and `--branch`, or two of them, name one repository | Keep one for the repository |
| `repo ... has no branch ... on its remote` | The branch is not pushed, or is misspelled | Check it with `git ls-remote` |
| `pull request #N of ... comes from a fork` | The branch is in the fork, not in the repository | `octok` takes only branches of the repository itself: fetch that fork by hand |
| `branch ... of ... is already checked out in ...` | Another workspace has that branch | Use that workspace, or remove it |
| `workspace ... already reviews other branches: ...` | The name is taken by a review with other choices | `octok rm <name>`, or choose another `--name` |
| `gh pr view ... failed` | The pull request does not exist, or `gh` is not signed in | Check the number, run `gh auth status` |

Each of them, with its exit code, is explained in [Troubleshooting](../troubleshooting.md).
Here are three of them as they are printed:

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
