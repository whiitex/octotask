# A Docker Compose per workspace

Give each workspace its own containers, its own database and its own ports, when a repository starts its services with Docker Compose.

A backend often has a `compose.yaml` next to its code, for the database and the other services it needs.
With two workspaces running at the same time, both checkouts would start the same containers on the same ports.
Docker Compose names a project after its folder, and the checkout of a repository has the same folder name in every workspace, so the second `octok up` would take over the containers of the first.
[Env values](../commands/up.md#shared-values) give each workspace its own project name and its own ports, written once in the project.
The examples use the project `shop`, with a backend that runs `uv run main.py` and has a `compose.yaml`, and a frontend that calls it.

## The compose file

Docker Compose replaces `${NAME}` in `compose.yaml` with the variable of the environment, and reads `COMPOSE_PROJECT_NAME` as the name of the project.
Write the host ports as variables:

```yaml
services:
  db:
    image: postgres:17
    environment:
      POSTGRES_PASSWORD: postgres
    ports:
      - "${DB_PORT}:5432"
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "postgres"]
      interval: 2s
```

Each workspace then gets its own containers and its own volumes, so each branch has a database of its own.

## The commands and the values

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok repo set backend --run 'docker compose up -d --wait && exec uv run main.py' --env 'COMPOSE_PROJECT_NAME=backend-{{ workspace.slug }}' --env DB_PORT=5432 --env PORT=9008
updated `shop-backend (backend)` in shop
$ octok repo set frontend --run 'npm run dev' --env 'VITE_API_URL=http://localhost:{{ repos.backend.env.PORT }}'
updated `shop-frontend (frontend)` in shop
```

- `docker compose up -d --wait` starts the services in the background, and returns once their health checks pass.
  The backend then starts with its database ready.
- `exec` makes the backend the command that `octok up` follows and stops.
- `{{ workspace.slug }}` is the name of the workspace in lowercase, with only letters, digits, `_` and `-`, as Docker Compose wants for a project name.
- The backend reads `$PORT` and `$DB_PORT`, and the frontend reads `$VITE_API_URL`: each one reads the values of its own workspace.

## A second workspace beside the first

Two workspaces get two project names by themselves, but they would still ask for the same ports.
Give the second one its own:

```console
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok repo set backend --env PORT=9009 --env DB_PORT=5433 --branch feat/cart
updated `shop-backend` in workspace `feat-cart` of shop
```

Now `octok up --branch feat/cart` starts the project `backend-feat-cart`, with the database on port 5433 and the backend on 9009, and its frontend calls `http://localhost:9009`.
Every other workspace keeps 5432 and 9008.
`docker compose ls` lists the project of each workspace.

## Stopping and removing the containers

The containers run in the background, so they outlive `octok up`: `Ctrl-C` stops the backend, and the database keeps its data for the next run.
`octok rm` does not touch them either.
Stop or remove them with Docker Compose, from anywhere:

```sh
docker compose -p backend-feat-cart stop
docker compose -p backend-feat-cart down -v
```

`down -v` deletes the containers and the volumes, with the data of the database of that branch.

## A backend on another machine

When the backend [lives on another machine](remote-machine.md), its containers run there too, with the same values.
`localhost` is then the other machine for the backend, but not for a frontend on your laptop: give the frontend the address of the machine, for example with `--env 'VITE_API_URL=http://api-dev-1:{{ repos.backend.env.PORT }}'`.

## Related pages

- [Running](../commands/up.md), for `octok up` and the env values.
- [Repositories on another machine](remote-machine.md).
