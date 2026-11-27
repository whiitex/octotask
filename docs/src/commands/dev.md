# Dev changes

How `octok dev` saves the local-only edits and untracked files that a repository needs to run on your machine, puts them into every new workspace and keeps them out of every commit.

Typical **dev changes** are a port, a database name, a line turned off or a `.env` file.

## How to use it

1. Commit your real work first: whatever is not committed counts as a dev change.
2. Make the dev changes in a checkout of a [workspace](../concepts.md#workspace), then run `octok dev save`.
3. Name each file that git ignores once: `octok dev save --file .env`.
4. Check what is saved with `octok dev show`.
5. Create workspaces as usual with `octok branch`, `octok checkout` or `octok pr review`: each new checkout gets the dev changes.
6. To change or remove them, edit the checkout and run `octok dev save` again.

## Options

| Command | What it does |
|---|---|
| `octok dev save [<repo>...] [-b <branch>]` | Saves the dev changes of the named repositories (keys or aliases), or of every repository with a checkout in the workspace. |
| `... --file <path>` | Also saves a file that git ignores, by its path in the checkout. Can be repeated. |
| `octok dev show [<repo>...]` | Shows the saved files of each repository, and the diff of the tracked ones against git. Works from any folder, with the default project or `-p`. |
| `... --allow-stale-dev` | With `octok branch`, `octok checkout` or `octok pr review`: creates the workspace even when some dev changes no longer apply. |

## Rules

- **What is saved:** every uncommitted change to a tracked file, every file that git does not track, and an ignored file only when named with `--file`.
  Once saved, an ignored file stays saved while it is in the checkout.
  A folder is not a file: name each file.
- **Which workspace:** the one of `--branch`, or the one that contains the current folder, with its project whatever `-p` says.
- **One set per repository:** each save replaces the whole set of the repository, from whichever workspace it runs, so two workspaces cannot have different dev changes at once.
  A dev change taken out of the checkout is no longer saved.
- **New workspaces:** every new checkout gets the changed files with their changes, and the other files where it does not have them.
  Existing workspaces keep their files.
- **Hidden from git:** `git status` and `git diff` do not show them, and the saved untracked files are ignored in every checkout.
  A commit of a tracked file that has dev changes holds your edit only, and the dev changes stay on disk.
  `octok` does it with a git filter named `octok-dev` in the clone, which you never run yourself.
- **Edits on the same lines:** an edit to the very lines of a dev change makes `git status`, `git diff`, `git add` and `git commit` fail on that file, and stage nothing.
  If the edit is a new dev change, run `octok dev save`; otherwise undo it.
  An older checkout fails the same way after a save from another workspace: run `octok dev save` there, or edit the file to match.
- **When they no longer apply:** when the branch changed the same lines as a dev change, or lacks the file, the workspace is not created and the error names the file.
  `--allow-stale-dev` creates it with that file as git has it: make the dev changes again there, then run `octok dev save`.
- **Where they are kept:** on the machine of the repository, so a `.env` never leaves it: see [Locations and SSH](locations.md).
  A workspace with a [location of its own](locations.md#example-a-workspace-with-its-own-location) on another machine saves there: only workspaces later placed on that machine get it, and `octok dev show` does not show it.
  They are deleted with the repository, with the project, and when the repository moves to another machine.
- **Secrets:** `octok dev show` and the [JSON documents](../agents/json-output.md) never print the content of an untracked file.
- **Requirements:** `git` and a POSIX shell on the machine of the repository.

| Exit code | When |
|---|---|
| 2 | Not inside a workspace and no `--branch`, or a `--file` path outside the checkout. Nothing is saved. |
| 3 | A named repository is not registered or has no checkout in the workspace. |
| 3 | No saved checkout has the `--file` file. The other dev changes are saved all the same. |

## Example: save the dev changes

The backend has `run(port=8000)` in `main.py`, and you run it on port 9005 with a token in a `.env` file that git ignores:

```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
$ octok repo add git@github.com:acme/shop-backend.git --alias backend
added `shop-backend (backend)` to shop: git@github.com:acme/shop-backend.git
$ octok dev show
shop-backend (backend)
  no dev changes
$ octok branch feat/cart
created workspace `feat-cart` in shop
/Users/you/.octotask/workspaces/shop/feat-cart
$ cd "$OCTOTASK_HOME/workspaces/shop/feat-cart/shop-backend"
$ cat main.py
from shop import run

run(port=8000)
$ sed -i.bak 's/port=8000/port=9005/' main.py && rm main.py.bak
$ echo '.env' >> "$(git rev-parse --git-path info/exclude)"
$ echo 'TOKEN=secret' > .env
$ git status --short
 M main.py
$ octok dev save
shop-backend (backend)
  changed  main.py
$ octok dev save --file .env
shop-backend (backend)
  changed  main.py
  new      .env
$ git status --short
$ git diff
$ cat main.py
from shop import run

run(port=9005)
$ octok dev show
shop-backend (backend)
  changed  main.py
  new      .env

diff --git a/main.py b/main.py
index b194827..b8c8f4c 100644
--- a/main.py
+++ b/main.py
@@ -1,3 +1,3 @@
 from shop import run

-run(port=8000)
+run(port=9005)
```

After the save, `main.py` on disk still has port 9005, and git sees nothing.

## Example: what is refused

```console
$ octok dev save --file nothing.txt
error: no checkout has the file `nothing.txt`
$ octok dev save --file ../x
error: invalid file `../x`: must be a path inside the checkout
$ cd /
$ octok dev save
error: not inside a workspace: pass its branch, as in `octok dev save --branch <branch>`
```

## Example: a new workspace

```console
$ octok branch feat/search
created workspace `feat-search` in shop
/Users/you/.octotask/workspaces/shop/feat-search
$ cd "$OCTOTASK_HOME/workspaces/shop/feat-search/shop-backend"
$ cat main.py
from shop import run

run(port=9005)
$ cat .env
TOKEN=secret
$ git status --short
```

## Example: commit next to them

The commit changes the import line, and the port on disk stays 9005:

```console
$ sed -i.bak 's/import run/import run, debug/' main.py && rm main.py.bak
$ git status --short
 M main.py
$ git diff
diff --git a/main.py b/main.py
index b194827..1914aba 100644
--- a/main.py
+++ b/main.py
@@ -1,3 +1,3 @@
-from shop import run
+from shop import run, debug

 run(port=8000)
$ git add main.py
$ git commit -qm "Import debug"
$ git show --format=%s
Import debug

diff --git a/main.py b/main.py
index b194827..1914aba 100644
--- a/main.py
+++ b/main.py
@@ -1,3 +1,3 @@
-from shop import run
+from shop import run, debug

 run(port=8000)
$ cat main.py
from shop import run, debug

run(port=9005)
$ git status --short
```

## Example: an edit on the same lines

The new port is a dev change too, so saving it fixes the error:

```console
$ sed -i.bak 's/port=9005/port=9006/' main.py && rm main.py.bak
$ git status --short
octok: main.py changed where its dev changes are, so they cannot be taken out of it.
octok: if this is a dev change too, save it with `octok dev save`.
error: external filter 'sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f' failed 1
error: external filter 'sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f' failed
fatal: main.py: clean filter 'octok-dev' failed
$ git add main.py
octok: main.py changed where its dev changes are, so they cannot be taken out of it.
octok: if this is a dev change too, save it with `octok dev save`.
error: external filter 'sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f' failed 1
error: external filter 'sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f' failed
fatal: main.py: clean filter 'octok-dev' failed
$ octok dev save
shop-backend (backend)
  changed  main.py
  new      .env
$ git status --short
$ cat main.py
from shop import run, debug

run(port=9006)
```

## Example: a branch where they no longer apply

A teammate pushes a branch that uses another port, so `main.py` comes as git has it and `.env` is copied in:

```console
$ git clone -q git@github.com:acme/shop-backend.git "$HOME/teammate"
$ cd "$HOME/teammate"
$ git switch -q -c feat/port origin/staging
$ sed -i.bak 's/port=8000/port=8080/' main.py && rm main.py.bak
$ git commit -qam "Use port 8080"
$ git push -q origin feat/port
$ octok checkout feat/port
error: the dev changes of `shop-backend` no longer apply to: main.py
The branch changed the same lines, or lacks those files. `--allow-stale-dev` creates the workspace with those files as they are in git: make the dev changes again there, then run `octok dev save`.
$ octok checkout feat/port --allow-stale-dev
created workspace `feat-port` in shop
  branch from the remote: shop-backend
/Users/you/.octotask/workspaces/shop/feat-port
$ cd "$OCTOTASK_HOME/workspaces/shop/feat-port/shop-backend"
$ cat main.py
from shop import run

run(port=8080)
$ cat .env
TOKEN=secret
$ sed -i.bak 's/port=8080/port=9005/' main.py && rm main.py.bak
$ octok dev save
shop-backend (backend)
  changed  main.py
  new      .env
$ git status --short
$ octok dev show backend
shop-backend (backend)
  changed  main.py
  new      .env

diff --git a/main.py b/main.py
index 78a4c4a..b8c8f4c 100644
--- a/main.py
+++ b/main.py
@@ -1,3 +1,3 @@
 from shop import run

-run(port=8080)
+run(port=9005)
```

## Example: remove them

```console
$ git show HEAD:main.py > main.py
$ rm .env
$ octok dev save
shop-backend (backend)
  no dev changes
$ octok dev show
shop-backend (backend)
  no dev changes
$ cd ..
$ octok dev save nothing
error: repo `nothing` is not registered
$ octok dev save backend --branch feat/port
shop-backend (backend)
  no dev changes
$ octok rm feat/port
removed workspace `feat-port` from shop
```

## Related pages

- [Workspaces](workspaces.md), for the commands that create checkouts.
- [Reviewing a pull request](pr-review.md), for `octok pr review`.
- [Locations and SSH](locations.md), for repositories on another machine.
- [JSON output](../agents/json-output.md), for the documents of `octok dev`.
