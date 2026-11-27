# Octotask

**One feature, several repositories, one command.**

`octok` runs the same git operation on every repository of a project.

A feature often touches several repositories at once: a frontend, a backend, an agent.
Working on it means creating the same branch in each of them, keeping them up to date and remembering where each one stands.
`octok` groups those repositories into a **project** and gives each feature a **workspace**: one folder with a checkout of every repository, all on the same branch.

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
shop-backend  shop-frontend

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

## Why try it

- **One command for all repos.** `octok branch feat/cart` creates the branch everywhere, from a freshly fetched base.
- **A teammate's branch in one command.** `octok checkout feat/login` builds the workspace of a branch that already exists.
- **Pull requests side by side.** `octok pr review` puts each repository on the pull request or branch you choose.
- **Isolated workspaces.** Each workspace is a set of `git worktree`s, so features stay open side by side with no stashing.
- **One command to start everything.** `octok up` starts every repository that has a run command and stops them all at `Ctrl-C`.
- **Plain git underneath.** Your own `git`, credentials, SSH config and hooks apply.
- **Repos on other machines.** A repository can live on a VM reached over `ssh`: `octok` runs the same commands there.
- **Dev changes that follow you.** Local-only edits and files such as `.env` are saved once, put into every new workspace and kept out of every commit.
- **Ready for coding agents.** Every command prints JSON with `--json`, and a skill teaches an agent to drive `octok`.

## Installation

Requires macOS or Linux and `git` 2.37 or newer.
`gh`, signed in, is needed only for `octok pr review`, `ssh` only for repositories on another machine, and one of `code`, `cursor`, `windsurf` or `codium` only for `octok open`.

From a release, for example on Apple silicon:

```sh
curl -fsSL https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz | tar -xz
sudo mkdir -p /usr/local/bin
sudo mv octok-aarch64-apple-darwin/octok /usr/local/bin/
```

From source, with [Rust](https://rustup.rs) 1.89 or newer:

```sh
git clone https://github.com/whiitex/octotask.git
cd octotask
make install
```

The other platforms, the checksums and what to do after installing are on the [Installation page](https://whiitex.github.io/octotask/getting-started/installation.html).

## Examples

The [Quick start](https://whiitex.github.io/octotask/getting-started/quick-start.html) walks through a first project and a first workspace.

### A teammate's existing branch

```console
$ cd "$(octok checkout feat/login)"
created workspace `feat-login` in shop
  branch from the remote: shop-backend
  new branch from the base: shop-frontend
```

Repositories that have the branch get it, the others start from their base, so the workspace is always complete.

### Reviewing a pull request across repositories

With a third repository, `shop-agent` with the alias `agent`, the backend on pull request #12 and the agent on the branch `fix/prompt`:

```console
$ cd "$(octok pr review --pr backend=12 --branch agent=fix/prompt)"
shop-backend (backend): pull request #12 is the branch `feat/login`
created workspace `pr-backend-12` in shop
  branch from the remote: shop-backend, shop-agent
  new branch from the base: shop-frontend
$ octok up
$ octok rm pr-backend-12
```

See [Reviewing a pull request](https://whiitex.github.io/octotask/guides/review-pull-request.html).

### Starting everything

```console
$ octok repo set frontend --setup 'npm ci' --run 'npm run dev -- --port 3000'
$ octok repo set backend --run 'uv run main.py' --env PORT=9005
$ octok repo set frontend --env 'VITE_API_URL=http://localhost:{{ repos.backend.env.PORT }}'
$ octok up
```

Every repository with a run command starts in its checkout, after its setup command the first time, the output is shown together with the name in front, and `Ctrl-C` stops them all.
A value shared between repositories, such as the port of the backend, is written once and read by the others, and a workspace can override it to run beside another one.
See [Running](https://whiitex.github.io/octotask/commands/up.html).

### Opening the editors

```console
$ octok open cursor --branch feat/login
```

Every repository of the workspace opens in its own window of `code`, `cursor`, `windsurf` or `codium`, and one on another machine opens through the Remote-SSH extension of the editor.
Pass repositories to open only those, and omit `--branch` inside a workspace.
See [Opening in an editor](https://whiitex.github.io/octotask/commands/open.html).

### Use with AI agents

A skill tells a coding agent when to use `octok`, the usual sequence of commands and what it must never do without asking you.

```sh
claude plugin install octotask --marketplace whiitex/octotask

codex plugin marketplace add whiitex/octotask
codex plugin add octotask@octotask
```

With `--json` every command prints its result as JSON on stdout and never asks a question, and its exit code says how it failed.
See [Driving octok from an agent](https://whiitex.github.io/octotask/agents/overview.html) and [JSON output](https://whiitex.github.io/octotask/agents/json-output.html).

## Documentation

- The site: <https://whiitex.github.io/octotask/>.
- In your terminal: `octok docs` lists the pages and `octok docs <topic>` prints one.
- For agents: <https://whiitex.github.io/octotask/llms.txt>.

Good next pages:

- [Concepts](https://whiitex.github.io/octotask/concepts.html)
- [Command reference](https://whiitex.github.io/octotask/reference/commands.html)
- [Repositories on another machine](https://whiitex.github.io/octotask/guides/remote-machine.html)
- [Dev changes](https://whiitex.github.io/octotask/commands/dev.html)
- [Troubleshooting](https://whiitex.github.io/octotask/troubleshooting.html)

## Contributing

Build, test and release instructions are on the [Development page](https://whiitex.github.io/octotask/development.html).

## License

Released under the [MIT license](LICENSE).
