# Opening in an editor

How `octok open` opens the repositories of a workspace in your editor, one window each, wherever they live.

## How to use it

1. For a repository on [another machine](locations.md), install the Remote-SSH extension in the editor, and make `ssh <host>` connect by itself: see [Repositories on another machine](../guides/remote-machine.md).
2. Open the workspace with `octok open <editor>` from any folder inside it, or with `octok open <editor> --branch <branch>` from anywhere.
   Name repositories to open only those: `octok open code frontend`.

## Options

| Command | What it does |
|---|---|
| `octok open <editor> [<repo>...] [-b <branch>]` | Opens the named repositories (keys or aliases), or every repository of the project, of the workspace of `--branch` or of the current folder. |

## Rules

- **Editors:** `code` (Visual Studio Code), `cursor`, `windsurf` and `codium`.
  Any other name is a usage error, with exit code 2, and the message lists the four.
- **Which workspace:** without `--branch`, the one that contains the current folder, with its project whatever `-p` says.
- **A window per repository:** a repository on this machine opens as `<editor> <path>`, and one on another machine as `<editor> --remote ssh-remote+<host> <path>`, with its folder there.
  The editor always runs on this machine.
- **An open folder is focused:** `octok` does not ask for a new window, so a folder already open comes to the front.
- **Checked first:** the workspace, the repositories, their checkouts and the `ssh` connections are checked before any window opens.
- **Output:** one line per repository, as `octok fetch` prints: the key, the alias in parentheses, `opened` and the folder, as `<host>:<path>` on another machine.
  `--json` prints one document: see [JSON output](../agents/json-output.md#octok-fetch-octok-pull-octok-open).
- **A failed window** does not stop the others: its line says `failed`, and stderr has `error: <name>: <error>`.
- **A missing editor** stops the command at the first repository with `` `<editor>` not found: install it or add it to PATH ``.
  On macOS, run "Shell Command: Install 'code' command in PATH" from the command palette of Visual Studio Code, or "Install 'cursor' command" in Cursor.

| Exit code | When |
|---|---|
| 0 | Every window opened. |
| 2 | Unknown editor, or not inside a workspace and no `--branch`. |
| 3 | Unknown workspace or repository, or a repository without a checkout in the workspace. |
| 6 | A window failed to open. |
| 7 | The editor is not installed, or a machine does not answer over `ssh`, with the error of `ssh`. |

## Example: open a workspace

The backend lives on `api-dev-1`, so Cursor opens it through Remote-SSH:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend --location api-dev-1
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
cloned at api-dev-1:/home/ubuntu/.octotask
$ octok checkout feat/login
created workspace `feat-login` in shop
  branch from the remote: shop-frontend, shop-backend
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend
/Users/you/.octotask/workspaces/shop/feat-login
$ octok open cursor --branch feat/login
shop-frontend (frontend)  opened /Users/you/.octotask/workspaces/shop/feat-login/shop-frontend
shop-backend (backend)    opened api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend
```

## Related pages

- [Workspaces](workspaces.md), for the workspace and its branch.
- [Locations and SSH](locations.md), for repositories on another machine.
- [Running](up.md), to start the repositories once they are open.
