# Locations and SSH

How a repository, a project or a workspace is placed on another machine over `ssh`, how `octok` talks to that machine, and what it refuses.

## How to use it

1. Check that `ssh <host>` logs in without a question, and that the machine has `git` 2.37 or newer, a POSIX shell and its own access to the git remotes.
2. Put a project there with `octok project init shop --location api-dev-1`, or one repository with `octok repo add <url> --location <loc>`.
3. Put one whole workspace on one machine with `octok branch <branch> --location <loc>`.
4. If the ssh key has a passphrase, type it once with `octok set-ssh-passphrase <host>`.
5. Work as usual: [`octok status`](status.md), [`octok pull`](sync.md), [`octok up`](up.md) and [`octok dev`](dev.md) reach every machine.

For a step by step setup see [Repositories on another machine](../guides/remote-machine.md).

## Options

| Location | Where the folders are |
|---|---|
| `local` | This machine, under `~/.octotask`. |
| `<host>` | `.octotask` in the home of the user on `<host>`. |
| `<host>:<path>` | `<path>` on `<host>`: under the home when relative or starting with `~/`, as it is when absolute. |

| Level | Set | Remove |
|---|---|---|
| Workspace | `octok branch <branch> --location <loc>`, `octok checkout <branch> --location <loc>`, [`octok pr review --location <loc>`](pr-review.md) | `octok rm <branch>` |
| Repository | `octok repo add <url> --location <loc>`, `octok repo set <repo> --location <loc>` | `octok repo set <repo> --no-location` |
| Project | `octok project init <name> --location <loc>`, `octok project set --location <loc>` | `octok project set --no-location` |

`octok set-ssh-passphrase [<host>]` adds the keys that `ssh` uses for `<host>` to the ssh agent, or those of every other machine of the project without `<host>`.

## Rules

- **Levels:** a repository lives at the first location set by its workspace, itself or its project, and on this machine when none is.
  `local` counts as a value, so a project on a VM can keep one repository here.
- **Hosts:** `<host>` is any `ssh` destination, such as a `Host` of `~/.ssh/config` or `user@host`, with only letters, digits, `.`, `_`, `-` and `@`, not starting with `-`.
  User, port, key and jump hosts come from `~/.ssh/config`; since `:` always starts the path, give another port its own `Host`.
- **Asked at a terminal:** `octok project init` and `octok repo add` ask for the location when no flag gives it, listing the machines of `~/.ssh/config` first.
- **Checked when set:** `octok` connects, creates the folder with `mkdir -p` and checks that `git` runs, but not its version, before saving anything.
  The project file keeps the absolute path, such as `location: api-dev-1:/home/ubuntu/.octotask`, so a change of the home moves nothing.
- **Same layout:** the remote folder holds `cache/<project>/<repo>.git`, `workspaces/<project>/<workspace>/<repo>` and `dev/<project>/<repo>`, like `~/.octotask`.
- **Workspaces span machines:** a workspace has one folder per machine, and `octok status`, `octok pull` and [`octok rm`](workspaces.md) reach all of them.
  `octok branch` and `octok checkout` list the remote checkouts and print the folder here, or `<host>:<path>` when there is none here.
- **A workspace with its own location** puts every repository there, cloning it when missing, until `octok rm <branch>`, which deletes the clones made only for it.
  Changing or adding `--location` on an existing workspace is refused with `workspace ... already exists at <path>`.
- **Moving** a repository needs no workspace using it, and a project no workspaces at all: remove them with `octok rm <branch>` first.
  `octok` clones at the new place and checks the base branch first, changing nothing if that fails, then deletes the old clone.
  The [dev changes](dev.md) of the repository are deleted, so save them again from the new machine.
- **Moving a project** moves the repositories that follow it, clones them again and prints one line each as [`octok fetch`](sync.md) does.
- **Leaving a machine:** [`octok project rm`](project.md) deletes the project folders on every machine, and the octok folder of a machine left empty.
- **Credentials on the machine:** give it its own key, or `ForwardAgent yes` in `~/.ssh/config`.
  Git runs there with `GIT_TERMINAL_PROMPT=0`, so a missing credential fails at once.
- **Ssh:** `octok` runs the system `ssh`, and the calls of a run share one connection for 60 seconds through `~/.ssh/octok-<hash>`.
  Without a terminal, and always with `--json`, it adds `-o BatchMode=yes`, so a passphrase or a new host key fails at once instead of asking.
  Under `octok up`, `ssh` gives up on a machine silent for 15 seconds.
- **Passphrases:** `octok set-ssh-passphrase` finds the keys with `ssh -G` and hands them to `ssh-add`, which asks: `octok` never sees the passphrase.
  It prints `<host>: added <key> to the ssh agent`, or `<host>: ssh uses no key file for this machine`.
  Without `<host>` it uses the default project or `-p`, and prints ``project `<name>` has no repo on another machine`` when there is nothing to do.
  On macOS the passphrase goes into the keychain, which `octok` reads with `-o UseKeychain=yes`, and survives a reboot; on Linux it lasts as long as the agent.
- **Errors:** a connection error shows the message of `ssh` and what to check, usually a missing `Host`, a key without agent, or a host key never accepted: run `ssh <host>` once by hand.
  A failed remote command names the machine and the folder: ``error: `mkdir -p -- /srv/octok` failed in api-dev-1:~ (exit status: 1)``.

| Exit code | When |
|---|---|
| 2 | An invalid location, refused before connecting, or `octok set-ssh-passphrase --json` (`needs_terminal`). |
| 5 | An existing workspace given another location. |
| 6 | Moving a project: a repository failed to clone again. |
| 7 | The machine cannot be reached, or `ssh` cannot log in. |

## Example: a project on another machine

The frontend stays here and the agent has its own folder; `octok repo ls` shows locations only when a repository is elsewhere:

```console
$ octok project init shop --location api-dev-1
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
its repos live at api-dev-1:/home/ubuntu/.octotask
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
cloned at api-dev-1:/home/ubuntu/.octotask
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend --location local
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-agent.git --alias agent --location api-dev-1:work/octok
added `shop-agent (agent)` to shop: git@github.com:acme/shop-agent.git
cloned at api-dev-1:/home/ubuntu/work/octok
$ octok repo ls
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git   api-dev-1:/home/ubuntu/.octotask
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git  local
shop-agent (agent)        staging  git@github.com:acme/shop-agent.git     api-dev-1:/home/ubuntu/work/octok
$ grep -B2 '^location:' "$OCTOTASK_HOME/projects/shop.yaml"
name: shop
base: staging
location: api-dev-1:/home/ubuntu/.octotask
```

## Example: a workspace across machines

The backend and the agent of `feat-cart` live on `api-dev-1`, the frontend here:

```console
$ octok branch feat/cart
created workspace `feat-cart` in shop
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend
  shop-agent: api-dev-1:/home/ubuntu/work/octok/workspaces/shop/feat-cart/shop-agent
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok status
project: shop

repos:
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)  [api-dev-1:/home/ubuntu/.octotask]
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)  [api-dev-1:/home/ubuntu/work/octok]

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-backend (backend)    feat/cart  de08597  Fix order totals (5 hours ago)  [not on the remote]  [api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend]
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-agent (agent)        feat/cart  a1ea388  Tune the prompt (3 days ago)  [not on the remote]  [api-dev-1:/home/ubuntu/work/octok/workspaces/shop/feat-cart/shop-agent]
```

## Example: a workspace with its own location

The workspace `feat-search` has every repository on this machine:

```console
$ octok branch feat/search --location local
created workspace `feat-search` in shop
/Users/you/.octotask/workspaces/shop/feat-search
$ octok status
project: shop

repos:
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)  [api-dev-1:/home/ubuntu/.octotask]
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-agent (agent)        staging  a1ea388  Tune the prompt (3 days ago)  [api-dev-1:/home/ubuntu/work/octok]

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-backend (backend)    feat/cart  de08597  Fix order totals (5 hours ago)  [not on the remote]  [api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend]
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-agent (agent)        feat/cart  a1ea388  Tune the prompt (3 days ago)  [not on the remote]  [api-dev-1:/home/ubuntu/work/octok/workspaces/shop/feat-cart/shop-agent]
  feat-search  /Users/you/.octotask/workspaces/shop/feat-search
    shop-backend (backend)    feat/search  de08597  Fix order totals (5 hours ago)  [not on the remote]
    shop-frontend (frontend)  feat/search  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-agent (agent)        feat/search  a1ea388  Tune the prompt (3 days ago)  [not on the remote]
```

## Example: move a repository and a project

A move is refused while workspaces exist:

```console
$ octok repo set agent --location local
error: repo `shop-agent` is used by workspaces: feat-cart, feat-search
$ octok project set --location api-dev-1
error: project `shop` still has workspaces: feat-cart, feat-search
remove them first with `octok rm <branch>`
```

After removing them, the agent moves here:

```console
$ octok rm feat/search
removed workspace `feat-search` from shop
$ octok rm feat/cart
removed workspace `feat-cart` from shop
$ octok repo set agent --location local
updated `shop-agent (agent)` in shop
$ octok repo ls
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git   api-dev-1:/home/ubuntu/.octotask
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git  local
shop-agent (agent)        staging  git@github.com:acme/shop-agent.git     local
```

Then the agent follows the project again, and the whole project comes back here:

```console
$ octok repo set agent --no-location
updated `shop-agent (agent)` in shop
$ octok project set --no-location
repos of `shop` now live at local
shop-backend (backend)    fetched
shop-frontend (frontend)  fetched
shop-agent (agent)        fetched
$ octok repo ls
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git
shop-agent (agent)        staging  git@github.com:acme/shop-agent.git
```

## Example: set the passphrase of a key

The key is added once, and `--json` is refused because the command must ask:

```console
$ octok set-ssh-passphrase api-dev-1
api-dev-1: added /Users/you/.ssh/id_ed25519 to the ssh agent
$ octok set-ssh-passphrase --json
{"schema":1,"error":{"code":"needs_terminal","message":"`octok set-ssh-passphrase` asks at the terminal, which `--json` never does: run it without `--json`","command":"octok set-ssh-passphrase"}}
```

## Example: what is refused

A host with a space is refused before connecting, and a machine that refuses the connection fails with exit code 7:

```console
$ octok repo set frontend --location 'bad host'
error: invalid location `bad host`: the host may only contain letters, digits, `.`, `_`, `-` and `@`
$ octok repo set frontend --location unreachable
error: cannot run commands on `unreachable` over ssh
ssh: connect to host unreachable port 22: Connection refused
check that `ssh unreachable` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`
if its key has a passphrase, run `octok set-ssh-passphrase unreachable`
```

## Related pages

- [Repositories on another machine](../guides/remote-machine.md), a walk through the whole setup.
- [Running](up.md), for what happens to a command when the connection is lost.
- [Dev changes](dev.md), kept on the machine of the repository.
- [Troubleshooting](../troubleshooting.md).
