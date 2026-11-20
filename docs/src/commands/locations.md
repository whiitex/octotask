# Locations and SSH

How a repository, a project or a workspace is placed on another machine over `ssh`, how `octok` talks to that machine, and what it refuses.

## What a location is for

By default everything lives on this machine, under `~/.octotask`.
A **location** moves the [cache](../concepts.md#cache) and the checkouts of a repository to another machine, for example a VM where the backend runs.
`octok` still runs on this machine only: for every repository that lives elsewhere it calls `ssh <host> git ...`, and the same for the commands of [`octok up`](up.md) and the files of [`octok dev`](dev.md).
Your git operations, your status and your pulls work the same, with the other machine doing the work.
For a step by step setup see [Repositories on another machine](../guides/remote-machine.md).
The concept is defined in [concepts](../concepts.md#location).

## The three forms

A location is written in one of three ways:

| Location | Where the folders are |
|---|---|
| `local` | This machine. |
| `<host>` | The folder `.octotask` in the home of the user on `<host>`. |
| `<host>:<path>` | The folder `<path>` on `<host>`. A relative path, or one that starts with `~/`, is under the home of the user. An absolute path is used as it is. |

`<host>` is anything `ssh` accepts as a destination: a `Host` of `~/.ssh/config`, or `user@host`.
The user, the port, the key and the jump hosts come from your ssh configuration, so `octok` stores none of them.
To use another port, name a `Host` for it in `~/.ssh/config`: a `:` in a location always starts the path.
A host may contain only letters, digits, `.`, `_`, `-` and `@`, and may not start with `-`.

When you give a location on another machine, `octok` connects once, asks the machine for the home of the user, creates the folder with `mkdir -p` and checks that `git` runs there.
The project file always holds the absolute path that results, for example `location: api-dev-1:/home/ubuntu/.octotask`, so a later change of the home does not move your data.
A machine that cannot be reached fails the command before anything is saved.

On the other machine the folder has the same layout as `~/.octotask` here: `cache/<project>/<repo>.git`, `workspaces/<project>/<workspace>/<repo>` and `dev/<project>/<repo>`.
See [configuration files](../reference/configuration.md) for the files of the project.

## The three levels

The location of a repository is the first one set among these three:

1. the workspace, with `--location` on [`octok branch`](workspaces.md), [`octok checkout`](workspaces.md) or [`octok pr review`](pr-review.md): every repository of that workspace lives there;
2. the repository, with `octok repo add --location` or `octok repo set --location`;
3. the project, with `octok project init --location` or `octok project set --location`.

When none is set, the repository lives on this machine.
`local` is a value like any other, so it overrides the levels below it in this list: a project on a VM can keep one repository here with `--location local`.

| Where | Set | Remove |
|---|---|---|
| Project | `octok project init <name> --location <loc>`, `octok project set --location <loc>` | `octok project set --no-location` |
| Repository | `octok repo add <url> --location <loc>`, `octok repo set <repo> --location <loc>` | `octok repo set <repo> --no-location` |
| Workspace | `octok branch <branch> --location <loc>`, `octok checkout <branch> --location <loc>`, `octok pr review --location <loc>` | `octok rm <branch>` |

At a terminal, `octok project init` and `octok repo add` ask for the location when no flag gives it, and list the machines named in `~/.ssh/config` first: see [projects](project.md) and [repositories](repo.md).

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

The project lives on `api-dev-1`, the frontend overrides it with `local`, and the agent has a folder of its own in the home of the user.
`octok repo ls` shows the location column only when a repository is not on this machine.

## A workspace across machines

A workspace has one folder per machine, and every command that works on a workspace reaches all of them.
[`octok branch`](workspaces.md) and `octok checkout` print the folder on this machine, or `<host>:<path>` when the workspace has no folder here.
The lines under `created workspace` name the checkouts that are not on this machine.

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

[`octok status`](status.md), [`octok pull`](sync.md) and [`octok rm`](workspaces.md) go to every machine of the workspace.

### A workspace with its own location

`--location` on `octok branch`, `octok checkout` or `octok pr review` puts the whole workspace on one machine, whatever the repositories and the project say.
The clones are made on that machine when they are not there yet.
The choice is saved in the project file until `octok rm <branch>`, which also deletes the clones made only for that workspace.
Run the command again with another location on a workspace that already exists, or with `--location` on one that was made without it, and it is refused with `workspace ... already exists at <path>` and exit code 5.

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

## Moving a repository or a project

A repository changes location only while no workspace uses it, because its checkouts would be left behind.
The same holds for `octok project set --location` and `--no-location`: they are refused while the project has workspaces, on any machine.

```console
$ octok repo set agent --location local
error: repo `shop-agent` is used by workspaces: feat-cart, feat-search
$ octok project set --location api-dev-1
error: project `shop` still has workspaces: feat-cart, feat-search
remove them first with `octok rm <branch>`
```

Remove the workspaces first with [`octok rm <branch>`](workspaces.md), then move:

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

What a move does:

- The new location is tried first: `octok` connects, creates the folder and clones the repository there, and checks that the remote has the base branch.
  If that fails, nothing changes.
- The clone is then deleted on the old machine and made again on the new one.
- The [dev changes](dev.md) saved for the repository are deleted too: they were kept on the old machine, and a `.env` never leaves it.
  Save them again from a checkout on the new one.
- `octok repo set <repo> --no-location` goes back to the location of the project.

`octok project set --location <loc>` moves every repository that follows the project.
It then clones them again where they live now, prints one line per repository as [`octok fetch`](sync.md) does, and exits with code 6 if any of them failed.
A repository with a location of its own stays where it is.

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

### Leaving a machine

[`octok project rm`](project.md) deletes the folders of the project on every machine it uses.
When that was the last project on a machine, the octok folder of that machine goes too, so nothing is left behind.

## What the other machine needs

- `git` 2.37 or newer.
  `octok` checks that `git` runs there when you set the location, and not its version.
- A POSIX shell and the usual tools: `octok` runs `sh` scripts there for `octok up` and `octok dev`.
- Its own access to the git remotes: give it its own key, or set `ForwardAgent yes` for that host in `~/.ssh/config`.
  Git runs there with `GIT_TERMINAL_PROMPT=0`, so a missing credential fails at once instead of waiting for a password.
- A login that needs no question: a key that `ssh` finds, in an agent or without a passphrase.

## How `octok` runs commands over ssh

Every command on another machine is one `ssh` call to the system `ssh`, with these options:

```text
ssh -o ControlMaster=auto -o ControlPath=~/.ssh/octok-%C -o ControlPersist=60 [-o BatchMode=yes] -- <host> 'exec env GIT_TERMINAL_PROMPT=0 git -C <dir> <args>'
```

- **Connection sharing.**
  The commands of a run share one connection, kept open for a minute after the last one, through a control socket `~/.ssh/octok-<hash>`.
  One `octok branch` makes many short commands, and they do not each pay for a login.
- **Quoting.**
  The command is handed to the shell of the remote user as one string, with every word quoted, so names with spaces or quotes are safe.
  A directory is passed as `git -C <dir>`.
- **No input.**
  `ssh` gets an empty stdin, except under `octok up`, which keeps it as the pipe that stops the commands.
- **`BatchMode=yes`.**
  Without a terminal on stdin, and always with `--json`, `octok` adds `-o BatchMode=yes`: nobody can type a passphrase or accept a host key, so a command that would have to ask fails at once instead of waiting.
  At a terminal, `ssh` may ask.
- **macOS keychain.**
  On macOS `octok` also passes `-o UseKeychain=yes` (with `IgnoreUnknown=UseKeychain` for an `ssh` that is not Apple's), so a passphrase stored by `octok set-ssh-passphrase` is read from the keychain.
- **`octok up`.**
  The long-running commands of `octok up` add `-o ServerAliveInterval=5 -o ServerAliveCountMax=3`: `ssh` gives up on a machine that has not answered for 15 seconds.
  See [Running](up.md).

Your own options in `~/.ssh/config`, such as `ProxyJump`, `IdentityFile` or `User`, apply as usual.

## `octok set-ssh-passphrase`

```text
octok set-ssh-passphrase [<host>]
```

Type the passphrase of an ssh key once instead of at every command.
The command finds the key files that `ssh` uses for `<host>`, with `ssh -G`, and gives them to `ssh-add`, which asks for the passphrase at the terminal and keeps it in the ssh agent.
`octok` never reads the passphrase and never stores it.
With `<host>` it needs no project.
Without `<host>`, it does this for every other machine of the default project, or of the one named with `-p`, or prints ``project `<name>` has no repo on another machine``.
It prints one line per key, `<host>: added <key> to the ssh agent`, or `<host>: ssh uses no key file for this machine`.
On macOS the passphrase also goes into the keychain, so it survives a reboot.
On Linux it lasts as long as the ssh agent of your session.

The command works only by asking, so it is refused with `--json`, with the error code `needs_terminal` and exit code 2.

```console
$ octok set-ssh-passphrase api-dev-1
api-dev-1: added /Users/you/.ssh/id_ed25519 to the ssh agent
$ octok set-ssh-passphrase --json
{"schema":1,"error":{"code":"needs_terminal","message":"`octok set-ssh-passphrase` asks at the terminal, which `--json` never does: run it without `--json`","command":"octok set-ssh-passphrase"}}
```

## What is refused and what fails

```console
$ octok repo set frontend --location 'bad host'
error: invalid location `bad host`: the host may only contain letters, digits, `.`, `_`, `-` and `@`
$ octok repo set frontend --location unreachable
error: cannot run commands on `unreachable` over ssh
ssh: connect to host unreachable port 22: Connection refused
check that `ssh unreachable` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`
if its key has a passphrase, run `octok set-ssh-passphrase unreachable`
```

- An invalid location is refused before `octok` connects, with exit code 2.
- A machine that cannot be reached, or that `ssh` cannot log in to, fails with the message of `ssh`, followed by what to check, and exit code 7.
  The usual causes are a `Host` that is not in `~/.ssh/config`, a key with a passphrase and no agent, or a host key that is not accepted yet: run `ssh <host>` once by hand.
- A command that fails on the other machine fails with the name of the machine and the folder, as it does here: ``error: `mkdir -p -- /srv/octok` failed in api-dev-1:~ (exit status: 1)``, followed by what the command printed.
- A repository that is in use, or a project with workspaces, cannot move: see above.
- Without a terminal, a key with a passphrase that is not in the agent fails at once: run `octok set-ssh-passphrase <host>` in a terminal first.

## Related pages

- [Repositories on another machine](../guides/remote-machine.md), a walk through the whole setup.
- [Running](up.md), for what happens to a command when the connection is lost.
- [Dev changes](dev.md), kept on the machine of the repository.
- [Troubleshooting](../troubleshooting.md).
