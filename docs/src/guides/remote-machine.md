# Repositories on another machine

Keep some repositories on a VM or another machine, the rest on your laptop, and work on both sides with the same commands.

`octok` runs on your machine only, and reaches the other one with `ssh`: for a repository that lives there, it runs `ssh <host> git ...` instead of `git ...`.
The examples keep `shop-frontend` on the laptop and `shop-backend` on the machine `api-dev-1`, user `ubuntu`.

## How to do it

1. Check that `ssh api-dev-1 true` connects by itself, with no question.
   The user, the port and the key come from your `~/.ssh/config`, and `octok` stores none of them.
2. On the other machine, check that `git --version` is 2.37 or newer, and that the machine reaches the git remotes by itself: give it its own key, or set `ForwardAgent yes` for that host.
3. If the key has a passphrase, type it once with `octok set-ssh-passphrase api-dev-1`.
4. Give the repository its [location](../concepts.md#location): `octok repo add <url> --location api-dev-1`, or `octok repo set <repo> --location api-dev-1`.
   Put a whole project there with `octok project set --location api-dev-1`, or one workspace with `octok branch <branch> --location api-dev-1`.
5. Create the workspace as usual with `cd "$(octok branch feat/cart)"`: it gets one folder on each machine.
6. Work and push in the remote checkout over `ssh api-dev-1`, since that machine has the repository.
   `octok status`, `octok pull` and `octok rm` reach both machines.
7. Start everything with `octok up --branch feat/cart`, and stop it on both machines with `Ctrl-C`.
8. Edit both sides with `octok open <editor>`, which opens the remote checkout through the Remote-SSH extension.

The forms of a location, which level wins, moving a repository and what fails are in [Locations and SSH](../commands/locations.md).
See also [Running](../commands/up.md) and [Opening in an editor](../commands/open.md).

## Example: register a repository on the machine

The clone of the backend is made on `api-dev-1`, and the project file keeps the absolute path of its folder:

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

## Example: a workspace on two machines

`~/.octotask/workspaces/shop/feat-cart` holds `shop-frontend` on the laptop and `shop-backend` on the machine:

```console
$ cd "$(octok branch feat/cart)"
created workspace `feat-cart` in shop
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-cart/shop-backend
$ ls
shop-frontend
$ ssh api-dev-1 'ls .octotask/workspaces/shop/feat-cart'
shop-backend
```

`octok status` marks each checkout that is not here with its location:

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

## Example: commit and push on the machine

The remote checkout is a normal git working tree, and `octok` sees its commit over `ssh`:

```console
$ ssh api-dev-1 'cd .octotask/workspaces/shop/feat-cart/shop-backend && echo "def cart(): ..." > cart.py && git add cart.py && git commit --quiet -m "Add the cart endpoint" && git status --short --branch'
## feat/cart
```

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

`octok pull` then fetches and fast-forwards the checkouts of both machines:

```console
$ ssh api-dev-1 'cd .octotask/workspaces/shop/feat-cart/shop-backend && git push --quiet'
$ octok pull feat/cart
shop-frontend (frontend)  skipped: branch is not on the remote yet
shop-backend (backend)    up to date
```

## Example: start everything

Each command runs in its checkout, on the machine where the checkout lives:

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

The frontend reaches the backend at the address of the machine, not at `localhost`: set it in the run command or the [dev changes](../commands/dev.md) of the frontend.

## Example: a workspace on one machine

`--location` puts every repository of the workspace on the machine, and `octok rm` deletes the clones made only for it.
With no folder here, the workspace prints `<host>:<path>`:

```console
$ octok branch feat/x --location api-dev-1
created workspace `feat-x` in shop
  shop-frontend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-x/shop-frontend
  shop-backend: api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-x/shop-backend
api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-x
$ octok rm feat/x
removed workspace `feat-x` from shop
```

## Example: type the ssh passphrase once

`ssh-add` asks for the passphrase at the terminal, `Enter passphrase for /Users/you/.ssh/id_ed25519:`, and keeps it in the ssh agent:

```console
$ octok set-ssh-passphrase api-dev-1
api-dev-1: added /Users/you/.ssh/id_ed25519 to the ssh agent
```

Without a terminal, as in a script or under an agent, `ssh` cannot ask, so a command that needs the passphrase or a new host key fails at once: run `octok set-ssh-passphrase` yourself, once.

## Example: a connection that fails

The error carries the output of `ssh`, exits with code 7 and changes nothing:

```console
$ octok repo set frontend --location unreachable
error: cannot run commands on `unreachable` over ssh
ssh: connect to host unreachable port 22: Connection refused
check that `ssh unreachable` connects by itself: a location is what `ssh` takes, `<user>@<host>` or a `Host` of `~/.ssh/config`
if its key has a passphrase, run `octok set-ssh-passphrase unreachable`
```

1. Run `ssh <host>` yourself: if it asks for a passphrase or a host key, answer once or run `octok set-ssh-passphrase <host>`, and if it fails, fix your ssh configuration or the network.
2. Read the `ssh` line: `Connection refused` and `Connection timed out` are the network, `Permission denied (publickey)` the key, `Host key verification failed` the `known_hosts`.
3. Check that the name is a `Host` of `~/.ssh/config`, or `user@host`.
4. On the machine, check `git --version` and that `git ls-remote <url>` works: a clone fails there when the machine cannot reach the remote.

## Example: move a repository back

A repository or a project moves only while no workspace uses it:

```console
$ octok project set --location api-dev-1
error: project `shop` still has workspaces: feat-cart
remove them first with `octok rm <branch>`
$ octok rm feat/cart
removed workspace `feat-cart` from shop
$ octok repo set backend --no-location
updated `shop-backend (backend)` in shop
```

The move deletes the clone and the saved [dev changes](../commands/dev.md) on the old machine, and clones again on the new one.

## Related pages

- [Locations and SSH](../commands/locations.md), for every rule of locations and `octok set-ssh-passphrase`.
- [Running](../commands/up.md), for `octok up`.
- [Opening in an editor](../commands/open.md), for `octok open`.
- [A Docker Compose per workspace](compose-per-workspace.md).
- [Troubleshooting](../troubleshooting.md).
