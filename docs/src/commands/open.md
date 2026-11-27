# Opening in an editor

How `octok open` opens the repositories of a workspace in your editor, one window each, wherever they live.

## What it is for

A [workspace](../concepts.md#workspace) is one folder per repository, and some of them can be on [another machine](locations.md).
`octok open` opens each of them in its own window of an editor, so you do not look for the folders by hand.

```text
octok open <editor> [<repo>...] [--branch <branch>]
```

The editor is one of `code` (Visual Studio Code), `cursor`, `windsurf` and `codium`.
Any other name is refused as a usage error, with exit code 2, and the message lists the four.

- **Which workspace.**
  With `--branch` (or `-b`), the workspace of that branch.
  Without it, the workspace that contains the current folder, and the project of that workspace, whatever `-p` says.
  Anywhere else it is refused, with exit code 2, and tells you to pass `--branch`.
- **Which repositories.**
  Without `<repo>`, every repository of the project.
  With `<repo>` (a key or an alias, as many as you like), only those.
- **A window per repository.**
  A repository on this machine is opened as `<editor> <path>`.
  A repository on another machine is opened as `<editor> --remote ssh-remote+<host> <path>`, with its folder on that machine.
  The editor always runs on this machine.
- **Checked before anything opens.**
  A workspace that does not exist, a repository that is not registered, or a repository without a checkout in the workspace is refused with exit code 3 before any window opens.
  A machine that does not answer over ssh fails the whole command with the ssh error and exit code 7, also before any window opens.
- **An open folder is focused.**
  `octok` does not ask for a new window, so a folder that is already open in the editor comes to the front.

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

## Output

`octok open` prints one line for each repository, as `octok fetch` does: the key of the repository, followed by its alias in parentheses when it has one, then `opened` and the folder.
A folder on another machine is written as `<host>:<path>`.
With `--json`, it prints one document: see [JSON output](../agents/json-output.md#octok-fetch-octok-pull-octok-open).

## When a window does not open

- A repository that fails to open does not stop the others.
  A failed repository has `failed` after its name on stdout, and `error: <name>: <error>` on stderr, and the exit code is 6 when any of them failed.
- An editor that is not installed stops the command at the first repository, with exit code 7 and the error `` `<editor>` not found: install it or add it to PATH ``.
  On macOS, `code` and `cursor` are on PATH only after running "Shell Command: Install 'code' command in PATH" (Cursor: "Install 'cursor' command") from the editor's command palette.
- A repository on another machine needs the Remote-SSH extension in the editor, and `ssh <host>` that connects by itself: see [Repositories on another machine](../guides/remote-machine.md).

## Related pages

- [Workspaces](workspaces.md), for the workspace and its branch.
- [Locations and SSH](locations.md), for repositories on another machine.
- [JSON output](../agents/json-output.md#octok-fetch-octok-pull-octok-open), for the document of `--json`.
- [Running](up.md), to start the repositories once they are open.
