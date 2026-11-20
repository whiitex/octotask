# Repositories on another machine

Keep some repositories on a VM or another machine, the rest on your laptop, and work on both sides with the same commands.

Sometimes a repository cannot run on your laptop: the backend needs the GPU, the database or the network of a VM.
`octok` runs on your machine only, and it reaches the other machine with `ssh`: for a repository that lives there, it runs `ssh <host> git ...` instead of `git ...`.
A [location](../concepts.md#location) says where each repository lives.
The examples use the machine `api-dev-1`, user `ubuntu`, and the project `shop` with `shop-frontend` on the laptop and `shop-backend` on the VM.
Every command of this page that can run in a sandbox is checked against the real `octok`, with a fake machine standing for `api-dev-1`.

## Before you start

- `ssh api-dev-1` must connect by itself, with no question.
  A location is anything `ssh` takes: a `Host` of `~/.ssh/config`, or `user@host`.
  The user, the port and the key come from your ssh configuration, and `octok` stores none of them.
- The other machine needs `git` 2.37 or newer.
- The other machine must reach the git remotes by itself, because the clones are made and fetched there.
  Give it its own key, or set `ForwardAgent yes` for that host in `~/.ssh/config`.

Check the first one before anything else:

```console
$ ssh api-dev-1 true
```

## Set the location

The location of a repository is the first one set among these three:

1. the workspace, with `octok branch <branch> --location <location>`;
2. the repository, with `octok repo add --location` or `octok repo set --location`;
3. the project, with `octok project init --location` or `octok project set --location`.

When none is set, the repository lives on this machine.
A location is written `local`, `<host>` or `<host>:<path>`: with only the host, the folder is `.octotask` in the home of the user on the machine, and `octok` connects once to ask for that home.
See [Locations and SSH](../commands/locations.md).

Register the frontend here and the backend on the VM:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend --location api-dev-1
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
cloned at api-dev-1:/home/ubuntu/.octotask
$ octok repo ls
shop-frontend (frontend)  staging  git@github.com:acme/shop-frontend.git  local
shop-backend (backend)    staging  git@github.com:acme/shop-backend.git   api-dev-1:/home/ubuntu/.octotask
```

The clone of the backend was made on the VM, which is why the machine needs access to the remote.
The project file keeps the absolute path, `api-dev-1:/home/ubuntu/.octotask`, whatever you typed.
To put the whole project on the VM, use `octok project set --location api-dev-1` while it has no workspace, and `--no-location` to come back.

## What ends up where

Create the workspace as usual.
It has one folder per machine:

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend
$ ls
shop-frontend
$ ssh api-dev-1 'ls .octotask/workspaces/shop/feat-cart'
shop-backend
```

- On your machine, `~/.octotask/workspaces/shop/feat-cart` holds `shop-frontend`.
- On the VM, `~/.octotask/workspaces/shop/feat-cart` holds `shop-backend`, and its clone is in `~/.octotask/cache/shop`.
- `octok branch` and `octok checkout` print the folder on this machine.
  A workspace with no folder here, because every repository of it is on another machine, prints `<host>:<path>` instead.
- The dev changes of a repository, such as a `.env`, are kept on the machine where the repository lives and never leave it.

`octok status`, `octok pull` and `octok rm` reach both machines, and mark each checkout that is not here with its location:

```console
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)  [api-dev-1:/home/ubuntu/.octotask]

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/cart  de08597  Fix order totals (5 hours ago)  [not on the remote]  [api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend]
```

## Work in the remote checkout

The checkout on the VM is a normal git working tree.
Open a shell there with `ssh api-dev-1`, or run one command at a time:

```console
$ ssh api-dev-1 'cd .octotask/workspaces/shop/feat-cart/shop-backend && echo "def cart(): ..." > cart.py && git add cart.py && git commit --quiet -m "Add the cart endpoint" && git status --short --branch'
## feat/cart
```

`octok` sees the commit from here, because it reads the checkout over `ssh`:

```console
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)  [api-dev-1:/home/ubuntu/.octotask]

workspaces:
  feat-cart  /Users/you/.octotask/workspaces/shop/feat-cart
    shop-frontend (frontend)  feat/cart  1ec5189  Add cart page (2 days ago)  [not on the remote]
    shop-backend (backend)    feat/cart  966f9ed  Add the cart endpoint (10 minutes ago)  [not on the remote]  [api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend]
```

Push from the VM, with `git push` in that checkout, since the VM is the machine that has the repository: it needs its own key or a forwarded agent, as above.
`octok` never pushes.

```console
$ ssh api-dev-1 'cd .octotask/workspaces/shop/feat-cart/shop-backend && git push --quiet'
$ octok pull feat/cart
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    up to date
```

`octok pull` fetches and fast-forwards the checkouts of both machines.
The frontend has no remote branch yet, so it is skipped.

## Start everything with `octok up`

`octok up` starts every repository of the workspace that has a run command, wherever it lives, and shows all their output together:

```sh
octok repo set frontend --run 'npm run dev -- --port 3000'
octok repo set backend --run 'PORT=9005 uv run main.py'
octok up --branch feat/cart
```

```text
frontend | $ npm run dev -- --port 3000
backend  | $ PORT=9005 uv run main.py
backend  | Uvicorn running on http://0.0.0.0:9005
frontend | Local: http://localhost:3000/
^Cstopping...
backend  | stopped
frontend | stopped
```

- A command runs with `sh` in the checkout of its repository, on the machine where the checkout lives.
- Ctrl-C stops every repository on both machines.
  A command on the VM stops in the same way, and it also stops by itself when it has not heard from `octok` for 30 seconds, which is what a lost connection looks like from the VM.
- `octok` gives up on a machine that has not answered for 15 seconds.
- `octok up` shares one ssh connection among the commands of a run.

The frontend reaches the backend at the address of the VM, not at `localhost`: set the port and host in the run commands or in the [dev changes](../commands/dev.md) of the frontend.
See [Running](../commands/up.md).

## A workspace on one machine

`--location` on `octok branch`, `octok checkout` or `octok pr review` puts every repository of one workspace on a machine, whatever the repositories and the project say.
The clones are made there when they are not already.
The choice is remembered until `octok rm`, which also deletes the clones made only for that workspace:

```console
$ octok branch feat/x --location api-dev-1
created workspace `feat-x` in shop
  shop-frontend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-x/shop-frontend
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-x/shop-backend
api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-x
$ octok rm feat/x
removed workspace `feat-x` from shop
```

The workspace had no folder here, so the path printed is `api-dev-1:<path>`.

## The ssh passphrase

If the key of the VM has a passphrase, `ssh` asks for it at every command, and `octok` runs many.
Type it once with `octok set-ssh-passphrase`:

```console
$ octok set-ssh-passphrase api-dev-1
api-dev-1: added /Users/you/.ssh/id_ed25519 to the ssh agent
```

The command finds the key files `ssh` uses for the host and gives them to `ssh-add`, which asks for the passphrase and keeps it in the ssh agent.
`octok` never reads the passphrase and never stores it.
In a terminal you see the prompt of `ssh-add`, `Enter passphrase for /Users/you/.ssh/id_ed25519:`, before that line.
Without a host, it does this for every other machine of the project.
On macOS the passphrase also goes into the keychain, so it survives a reboot.
On Linux it lasts as long as the ssh agent of your session.

It asks, so it needs a terminal and is refused with `--json`.
Without a terminal, for example in a script or under an agent, `octok` runs `ssh` with `BatchMode=yes`: a command that would have to ask for a passphrase or a host key fails at once.
That is the error of the next section, and the fix is to run `octok set-ssh-passphrase` yourself, once.

## When the connection fails

An unreachable machine is an error with the output of `ssh` under it.
Here the host refuses the connection:

```console
$ octok repo set frontend --location unreachable
error: cannot run commands on `unreachable` over ssh
ssh: connect to host unreachable port 22: Connection refused
check that `ssh unreachable` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`
if its key has a passphrase, run `octok set-ssh-passphrase unreachable`
```

The command exits with code 7, and with `--json` it prints the error `ssh` with the `host` and the `stderr`.
Go through this list:

1. Run `ssh <host>` yourself.
   If it asks for a passphrase or a host key, answer it once, or run `octok set-ssh-passphrase <host>`.
   If it fails, the problem is in your ssh configuration or the network, not in `octok`.
2. Read the `ssh` line under the message: `Connection refused` and `Connection timed out` are the network, `Permission denied (publickey)` is the key, `Host key verification failed` is `known_hosts`.
3. Check that the name is a `Host` of `~/.ssh/config`, or `user@host`.
4. On the VM, check that `git --version` is 2.37 or newer and that `git ls-remote <url>` works there.
   A clone that fails on the VM fails because the VM cannot reach the remote.

Nothing was changed by the failed command above.
See [Troubleshooting](../troubleshooting.md) for the other errors.

## Moving a repository

A repository changes location only while no workspace uses it, because its checkouts would be left behind.
`octok project set --location` is refused for the same reason while the project has workspaces:

```console
$ octok project set --location api-dev-1
error: project `shop` still has workspaces: feat-cart
remove them first with `octok rm <branch>`
$ octok rm feat/cart
removed workspace `feat-cart` from shop
$ octok repo set backend --no-location
updated `shop-backend (backend)` in shop
```

Moving a repository deletes its clone and its saved [dev changes](../commands/dev.md) on the old machine, and makes the clone again on the new one.
`octok project rm` deletes the folders of the project on every machine, and when that was the last project there, the octok folder of that machine goes too.
