# Running

How `octok up` starts the repositories of a workspace together, shows their output side by side and stops them all at once.

## What it is for

A repository can say how it is started, with two shell commands: the **run command** starts it, such as `npm run dev`, and the **setup command** prepares a checkout before its first run, such as `npm ci`.
`octok up` runs them for every repository of a [workspace](../concepts.md#workspace), shows what they print in one stream with the name of the repository in front, and stays attached until they have all ended.
`Ctrl-C` stops every repository and every process they started.

## Setting the commands

The commands belong to a [repository](../concepts.md#repository) of the project, and are set with [`octok repo set`](repo.md):

| Command | What it does |
|---|---|
| `octok repo set <repo> --run <command>` | Sets the run command of a repository. `--no-run` removes it. |
| `octok repo set <repo> --setup <command>` | Sets the setup command of a repository. `--no-setup` removes it. |
| `octok repo set <repo> --run <command> --branch <branch>` | Sets the run command for the workspace of one branch only. `--no-run --branch <branch>` goes back to the command of the repository. |
| `octok repo set <repo> --env <name>=<value>` | Sets an env value of the commands of a repository, see [shared values](#shared-values). `--no-env <name>` removes it. With `--branch <branch>`, for the workspace of that branch only. |

`--run`, `--setup` and `--env` can be given together, and with the other options of `repo set`, except `--branch`: a workspace has its own run command and env values only.
Both commands are plain text for `sh`, so they can hold variables, `&&`, pipes and quotes.
They are saved in the project file, as `run` and `setup` of the repository, and `run` under `workspaces` for a workspace: see [configuration files](../reference/configuration.md).
They are left out of every JSON document of `octok`, which does not even say whether a repository has one; only `octok up` shows them, in the lines it prints before running each one.
Env values are never shown at all.
With `--branch`, the workspace does not have to exist yet: the command is used once a workspace of that branch is created.

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo set frontend --setup 'npm ci' --run 'npm run dev -- --port 3000'
updated `shop-frontend (frontend)` in shop
$ octok repo set backend --run 'uv run main.py' --env PORT=9005
updated `shop-backend (backend)` in shop
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok repo set backend --env PORT=9006 --branch feat/cart
updated `shop-backend` in workspace `feat-cart` of shop
$ grep -B3 -A5 '^workspaces' "$OCTOTASK_HOME/projects/shop.yaml"
    run: uv run main.py
    env:
      PORT: "9005"
workspaces:
  feat-cart:
    repos:
      shop-backend:
        env:
          PORT: "9006"
```

The backend reads its port from `$PORT`: the workspace `feat-cart` runs it on port 9006, and every other workspace on 9005.
The override lasts until [`octok rm feat/cart`](workspaces.md) removes the workspace, or until `octok repo set backend --no-env PORT --branch feat/cart`.

## Shared values

The values the repositories of a project share, such as the port of the backend that the frontend calls, are written once, as **env values** of the repository they belong to.
`octok up` passes the env values of a repository to its setup and run commands as environment variables.

- **Names** start with a letter or `_`, and contain only letters, digits and `_`, as in `PORT` or `VITE_API_URL`.
  A **value** is any text without a newline.
  `--env` takes everything after the first `=`, so `--env URL=http://x/?a=b` keeps the second `=`, and `--env DEBUG=` sets an empty value.
- **Per workspace.** With `--branch`, a value is set for the workspace of that branch only: it replaces the value of the repository with the same name, or adds a new name.
- **Templates.** A value can read other values with `{{ <variable> }}`:

  | Variable | Value |
  |---|---|
  | `{{ repos.<repo>.env.<name> }}` | The env value `<name>` of the repository `<repo>`, a key or an alias, in the same workspace. |
  | `{{ project.name }}` | The name of the project. |
  | `{{ workspace.name }}` | The name of the workspace, as in `feat-cart`. |
  | `{{ workspace.slug }}` | The name of the workspace in lowercase, with every character but letters, digits, `_` and `-` replaced by `-`: `feat/Cart.v2` gives `feat-cart-v2`. Use it where a name must be lowercase, as for [Docker Compose](../guides/compose-per-workspace.md). |

  Templates work only in env values. In the setup and run commands, read the variables as the shell does, as in `$PORT`.
  A value read through a template can use templates too, but values that read each other in a loop are refused.
- **Checked twice.** `octok repo set` refuses a name it cannot use and a variable it does not know, such as `{{ repo.backend.env.PORT }}`.
  `octok up` refuses a variable that is not defined in the workspace it starts, with exit code 3, before anything is started: a value is never replaced by an empty text.

```console
$ octok repo set frontend --env 'VITE_API_URL=http://localhost:{{ repos.backend.env.PORT }}'
updated `shop-frontend (frontend)` in shop
$ octok repo set frontend --env 'API_KEY={{ repos.backend.env.KEY }}'
updated `shop-frontend (frontend)` in shop
$ octok up --branch feat/cart
error: env `API_KEY` of repo `shop-frontend` uses `{{ repos.backend.env.KEY }}`, which is not defined
$ octok repo set frontend --no-env API_KEY
updated `shop-frontend (frontend)` in shop
$ octok repo set frontend --env 'API={{ repo.backend.env.PORT }}'
error: invalid env `API` of repo `shop-frontend`: uses an unknown variable
```

In the workspace `feat-cart`, the frontend gets `VITE_API_URL=http://localhost:9006`, and in every other workspace `http://localhost:9005`.

## Starting a workspace

```text
octok up [<repo>...] [--branch <branch>]
```

- **Which workspace.**
  With `--branch` (or `-b`), the workspace of that branch.
  Without it, the workspace that contains the current folder, so you can run `octok up` from any folder inside it, and the project of that workspace, whatever `-p` says.
  Anywhere else it is refused, with exit code 2, and tells you to pass `--branch`.
- **Which repositories.**
  Without `<repo>`, every repository of the workspace that has a run command, the one of the workspace when it has one.
  With `<repo>` (a key or an alias, as many as you like), only those, and each must have a run command.
- **Nothing starts unless all can.**
  A named repository with no run command, a repository to start whose checkout is missing, an env value that reads one that is not defined, or a workspace where no repository has a run command, is refused with exit code 3 before anything is started.
- **Where a command runs.**
  In the checkout of its repository in that workspace, with `sh`, on the machine where the checkout lives, with the env values of its repository in that workspace added to the environment.
  A repository without a run command is not started.
  Commands read an empty stdin.

```console
$ cd "$OCTOTASK_HOME/workspaces/shop/feat-cart"
$ octok up nothing
error: repo `nothing` is not registered
$ octok repo set backend --no-run
updated `shop-backend (backend)` in shop
$ octok repo set backend --no-run --branch feat/cart
updated `shop-backend` in workspace `feat-cart` of shop
$ octok up backend
error: repo `shop-backend` has no `run` command: set one with `octok repo set shop-backend --run <command>`
$ octok repo set frontend --no-run
updated `shop-frontend (frontend)` in shop
$ octok up
error: no repo of workspace `feat-cart` has a `run` command: set one with `octok repo set <repo> --run <command>`
$ octok up --branch feat/nope
error: no workspace for `feat-nope`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch
$ cd /
$ octok up
error: not inside a workspace: pass its branch, as in `octok up --branch <branch>`
```

## A run

This is a run by hand, with the commands set as above.
It cannot be checked by the documentation tests, because it needs a terminal and ends only when you stop it.
The text between `^C` and the end is what `octok` prints after `Ctrl-C`.

```text
$ octok up --branch feat/cart
frontend | $ npm ci
backend  | $ uv run main.py
backend  | Uvicorn running on http://0.0.0.0:9006
frontend | added 312 packages in 9s
frontend | $ npm run dev -- --port 3000
frontend | Local: http://localhost:3000/
^Cstopping...
backend  | stopped
frontend | stopped
$ echo $?
130
```

### Output

- Every line a repository prints, on stdout or stderr, goes to the stdout of `octok` with the alias or the key of the repository in front, padded so that the lines line up.
- A line that starts with `$` is a command that `octok` runs, the setup or the run command, so you can tell which one is running.
- What `octok` itself says goes to stderr: `stopping...` and the line that says how each repository ended.
  Redirect stdout to keep only what the repositories print.
- With `--json`, `octok up` prints one document per event instead: see [JSON output](../agents/json-output.md#octok-up).

### Setup

The setup command runs before the run command, in the same checkout.

- It runs until it succeeds once in that checkout, and again when its text changes.
  The text of the last setup that succeeded is kept in the git directory of the checkout, so a new workspace runs it once and the next `octok up` does not.
- When it fails, the repository is not started, and its line says how the setup ended.
  The other repositories go on.
- A repository without a setup command goes straight to its run command.

```text
backend  | $ pip install -r requirements.txt
backend  | ERROR: Could not find a version that satisfies the requirement nothing
backend  | ended (exit status: 1)
```

## How it ends

### A command that ends by itself

It does not stop the others.
`octok up` ends when every repository has ended.
Each ending is reported on stderr:

```text
backend  | ended (exit status: 3)
```

The exit code of `octok up` is 0 when every repository ended well, and 1 when any of them failed, whatever the code it failed with.
When a command ends, the processes it started are stopped too, so nothing it launched outlives it.

### Ctrl-C and signals

`Ctrl-C` stops every repository, prints `stopping...` once, and `octok up` then exits with code 130.
For each repository, the command and every process it started receive `SIGTERM`, and `SIGKILL` if they are still running 10 seconds later.
A repository that needed the `SIGKILL` is reported as `killed: it was still running 10 seconds after the stop`; the others as `stopped`.

The same stop happens by itself when `octok` receives `SIGTERM` or `SIGHUP`, as when its terminal is closed, and when `octok` is killed with `SIGKILL`: the supervision of each command notices that `octok` is gone.
No process is left behind.

| Exit code | When |
|---|---|
| 0 | Every repository ended with status 0. |
| 1 | At least one repository ended with another status, or a setup failed. |
| 130 | You stopped it, or `octok` got `SIGTERM` or `SIGHUP`. |

The refusals before the start have their own codes, as for any command: 2 for a usage error, 3 for something not found. They are listed in [JSON output](../agents/json-output.md).

## Repositories on another machine

A repository whose checkout is on [another machine](locations.md) is started there, over `ssh`, with the same setup, the same output and the same stop.
Two more things apply:

- `octok` sends a heartbeat to the machine every 2 seconds.
  A command that has not heard from `octok` for 30 seconds stops by itself, which is what a lost connection looks like from the other machine: nothing is left running there.
- On its side, `ssh` gives up on a machine that has not answered for 15 seconds, so `octok` does not wait for ever on a connection that died without closing.

## Fixed ports

`octok` does not choose ports, forward them or check them: a port is an env value or part of the command.
Two workspaces that run the same repository at the same time need different ports, so give one of them its own value with `--env <name>=<value> --branch <branch>`, as above.
A repository on another machine listens on that machine, so use its address there.
For the services of a `compose.yaml`, see [A Docker Compose per workspace](../guides/compose-per-workspace.md).

## Related pages

- [Repositories](repo.md), for `octok repo set`.
- [Workspaces](workspaces.md), for the workspace a command runs in.
- [JSON output](../agents/json-output.md#octok-up), for the events of `--json`.
- [Troubleshooting](../troubleshooting.md).
