# Running

How `octok up` starts the repositories of a workspace together, shows their output in one stream and stops them all with `Ctrl-C`.

## How to use it

1. Give each repository a **run command**, and a **setup command** if a checkout needs one before its first run: `octok repo set <repo> --run 'npm run dev' --setup 'npm ci'`.
2. Write the values the repositories share, such as a port, once as **env values**: `octok repo set backend --env PORT=9005`.
   Another repository reads them with a template: `--env 'API_URL=http://localhost:{{ repos.backend.env.PORT }}'`.
3. Give one workspace its own command or value with `--branch <branch>`, for example a different port.
4. Start the workspace with `octok up`, from any folder inside it, or with `octok up --branch <branch>` from anywhere.
5. Stop everything with `Ctrl-C`, on every machine.

## Options

| Command | What it does |
|---|---|
| `octok repo set <repo> --run <command>` | Sets the run command. `--no-run` removes it. |
| `octok repo set <repo> --setup <command>` | Sets the setup command. `--no-setup` removes it. |
| `octok repo set <repo> --env <name>=<value>` | Sets an env value. `--no-env <name>` removes it. |
| `... --branch <branch>` | Applies `--run`, `--no-run`, `--env` or `--no-env` to the workspace of that branch only, even before it exists. |
| `octok up [<repo>...] [-b <branch>]` | Starts the named repositories, or every one with a run command. |

## Rules

- **Commands** are plain `sh` text, saved in the [project file](../reference/configuration.md).
  They run in the checkout of the repository, on the machine where it lives, with an empty stdin and the env values in the environment.
  `octok` prints a command only in the `$` line before running it, and never prints env values.
- **Env values:** a name is letters, digits and `_`, not starting with a digit; a value is any text on one line.
  Templates work only in env values; in commands, read the variable as the shell does, as in `$PORT`.
- **Templates:** `{{ repos.<repo>.env.<name> }}`, `{{ project.name }}`, `{{ workspace.name }}` and `{{ workspace.slug }}`, the name in lowercase with every other character replaced by `-` (`feat/Cart.v2` gives `feat-cart-v2`).
  An unknown variable is refused by `octok repo set`, and a value not defined in the workspace or read in a loop by `octok up`.
- **Nothing starts unless all can:** a named repository without a run command, a missing checkout or an undefined value stops `octok up` before anything starts.
- **Setup** runs before the run command until it succeeds once in a checkout, and again when its text changes.
  A failed setup skips that repository only.
- **Output:** each line has the repository in front, on stdout.
  `octok`'s own lines, such as `stopping...` and how each repository ended, go to stderr.
  `--json` prints one event per line: see [JSON output](../agents/json-output.md#octok-up).
- **Ending:** a command that ends does not stop the others, and `octok up` returns when all have ended.
  `Ctrl-C`, `SIGTERM` or `SIGHUP` send `SIGTERM` to every command and what it started, then `SIGKILL` after 10 seconds.
  No process is left behind, also on [another machine](locations.md), where a command stops by itself 30 seconds after losing `octok`.
- **Ports:** `octok` does not choose or forward ports.
  Two workspaces running the same repository at once need different values, set with `--branch`.

| Exit code | When |
|---|---|
| 0 | Every repository ended with status 0. |
| 1 | A repository ended with another status, or a setup failed. |
| 2 | Not inside a workspace and no `--branch`. |
| 3 | Refused before the start: unknown repository or workspace, no run command, undefined value. |
| 130 | Stopped with `Ctrl-C`, `SIGTERM` or `SIGHUP`. |

## Example: set the commands

The backend runs on port 9005, and on 9006 in the workspace `feat-cart`:

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

## Example: share a value

The frontend reads the port of the backend, so it gets `http://localhost:9006` in `feat-cart` and `http://localhost:9005` elsewhere:

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

## Example: a run

`Ctrl-C` stops both repositories:

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

A repository that ends by itself or fails its setup, and one still running 10 seconds after the stop, are reported like this:

```text
backend  | ended (exit status: 1)
backend  | killed: it was still running 10 seconds after the stop
```

## Example: what is refused

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

## Related pages

- [Repositories](repo.md), for `octok repo set`.
- [Workspaces](workspaces.md), for the workspace a command runs in.
- [A Docker Compose per workspace](../guides/compose-per-workspace.md), for ports of services.
- [Troubleshooting](../troubleshooting.md).
