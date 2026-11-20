# octok

octok in the sandbox.

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend
added `shop-frontend (frontend)` to shop: git@github.com:acme/shop-frontend.git
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ cd "$(octok checkout feat/login)"
created workspace `feat-login` in shop
  branch from the remote: shop-frontend, shop-backend
$ ls
shop-backend
shop-frontend
$ octok status
project: shop

repos:
  shop-frontend (frontend)  staging  1ec5189  Add cart page (2 days ago)
  shop-backend (backend)    staging  de08597  Fix order totals (5 hours ago)

workspaces:
  feat-login  /Users/you/.octotask/workspaces/shop/feat-login
    shop-frontend (frontend)  feat/login  1695c74  Add login form (4 hours ago)
    shop-backend (backend)    feat/login  ec75932  Add login endpoint (3 hours ago)
```
