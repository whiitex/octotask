# A Docker Compose per workspace

Give each workspace its own containers, its own database and its own ports, when a repository starts its services with Docker Compose.

Docker Compose names a project after its folder, and a checkout has the same folder name in every workspace, so a second `octok up` would take over the containers of the first.
[Env values](../commands/up.md) give each workspace its own project name and its own ports, written once in the project.

## How to do it

1. Write the host ports of `compose.yaml` as variables, such as `"${DB_PORT}:5432"`.
2. Start the services in the run command, before the backend: `docker compose up -d --wait && exec uv run main.py`.
   `--wait` returns once the health checks pass, and `exec` makes the backend the command that `octok up` follows and stops.
3. Name the Compose project after the workspace, with the env value `COMPOSE_PROJECT_NAME=backend-{{ workspace.slug }}`: the slug is lowercase, as Compose wants.
4. Give every other workspace that runs at the same time its own ports, with `--env ... --branch <branch>`.
5. Stop or remove the containers with `docker compose -p <project>`: they outlive `octok up`, and `octok rm` does not touch them.

## Example: the compose file

Each workspace gets its own containers and volumes, so each branch has a database of its own:

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

## Example: the commands and the values

The backend reads `$PORT` and `$DB_PORT`, and the frontend `$VITE_API_URL`, each with the values of its own workspace:

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

## Example: a second workspace beside the first

The project names differ by themselves, but the ports must be set:

```console
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ octok repo set backend --env PORT=9009 --env DB_PORT=5433 --branch feat/cart
updated `shop-backend` in workspace `feat-cart` of shop
```

`octok up --branch feat/cart` now starts the project `backend-feat-cart`, with the database on 5433 and the backend on 9009, and its frontend calls `http://localhost:9009`.
Every other workspace keeps 5432 and 9008, and `docker compose ls` lists the project of each one.

## Example: stop and remove the containers

`Ctrl-C` stops the backend, and the database keeps running with its data, until you stop it from any folder:

```sh
docker compose -p backend-feat-cart stop
docker compose -p backend-feat-cart down -v
```

`down -v` deletes the containers and the volumes, with the data of the database of that branch.

## Example: a backend on another machine

When the backend [lives on another machine](remote-machine.md), its containers run there too, with the same values.
`localhost` is then that machine for the backend, but not for a frontend on your laptop, so give the frontend the address of the machine:

```sh
octok repo set frontend --env 'VITE_API_URL=http://api-dev-1:{{ repos.backend.env.PORT }}'
```

## Related pages

- [Running](../commands/up.md), for `octok up` and the env values.
- [Repositories on another machine](remote-machine.md).
