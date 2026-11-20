# Dev changes

How `octok dev` saves the local-only edits and untracked files that a repository needs to run on your machine, puts them into every new workspace and keeps them out of every commit.

## The problem

Some changes are needed to run a repository on your machine and must never be committed: a port, a database name, a line turned off, a `.env` file.
Without help, they stay in one checkout, you have to make them again in every new [workspace](../concepts.md#workspace), and they land in a commit the day you run `git commit -a`.

`octok` calls them **dev changes**.
Save them once, and every new workspace gets them, while git never sees them.

| Command | What it does |
|---|---|
| `octok dev save [<repo>...] [--file <path>]... [--branch <branch>]` | Saves the dev changes of a workspace from its checkouts, replacing the ones saved before. |
| `octok dev show [<repo>...]` | Shows what is saved for each repository: the files, and the changes to the tracked ones. |
| `octok branch <branch> --allow-stale-dev` | Creates a workspace even when some dev changes no longer apply: see [When they no longer apply](#when-they-no-longer-apply). |

The saved set belongs to a repository of the project, not to a workspace.
`--allow-stale-dev` is also an option of `octok checkout` and `octok pr review`.

## The usual use

The backend of the project has `run(port=8000)` in `main.py`, and you run it on port 9005 on your machine, with a token in a `.env` file that git ignores.

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

After the save, the file on disk is unchanged, and `git status` and `git diff` show nothing: git no longer sees the dev changes.
`octok dev show` does: it lists the files, and the diff of the tracked ones against what git has.

## What `octok dev save` stores

```text
octok dev save [<repo>...] [--file <path>]... [--branch <branch>]
```

For each repository, from its checkout in the workspace:

- **Every change to a tracked file that is not committed.**
  Commit your real work before saving: whatever is not committed counts as a dev change.
- **Every file that git does not track**, such as a new file that is not ignored.
- **A file that git ignores**, such as `.env`, only when you name it with `--file <path>`: the path is relative to the checkout, and the option can be repeated.
  An ignored file is often a build folder or a cache, so `octok` does not guess.
  Once saved, it stays saved for as long as it is in the checkout, and `--file` is not needed again.
  The path must be inside the checkout, or the command is refused with exit code 2 before anything is saved.
  The file must exist in at least one of the checkouts saved, or the command fails with exit code 3: the other dev changes of those checkouts are saved all the same.

In the example above, the first `octok dev save` saved `main.py` only, and the second one added `.env`.

The workspace is the one given by `--branch`, or the one that contains the current folder, with its project whatever `-p` says.
Outside a workspace and without `--branch`, the command is refused with exit code 2.
Without `<repo>`, every repository of the workspace is saved, those with a checkout only.
With `<repo>` (key or alias), only those, and each must have a checkout in the workspace, or the command is refused with exit code 3.

### Saving again replaces everything

What is saved follows the checkout: edit the files and run `octok dev save` again.
A dev change that you took out of the checkout is no longer saved, and saving from a checkout with no dev changes left removes them all.
The set is for the whole repository, so saving from one workspace replaces what every new workspace will get.

```console
$ octok dev save --file nothing.txt
error: no checkout has the file `nothing.txt`
$ octok dev save --file ../x
error: invalid file `../x`: must be a path inside the checkout
$ cd /
$ octok dev save
error: not inside a workspace: pass its branch, as in `octok dev save --branch <branch>`
```

## New workspaces get them

[`octok branch`](workspaces.md), `octok checkout` and [`octok pr review`](pr-review.md) put the dev changes of each repository in every checkout they create: the changed files with their changes, and the other files copied in, where the checkout does not have them yet.
The workspaces that already exist keep their files.

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

## Commits do not contain them

A tracked file with dev changes can still be edited and committed.
The commit holds your edit and not the dev changes, which stay in the file on disk.
`git status` and `git diff` do not show the dev changes either.
The files that git does not track are ignored by git in every checkout of that repository.

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

The commit changed the import line, and the port on disk is still 9005.

### How it works

`octok` sets up a git filter for the saved files in the clone of the repository, so it applies to every checkout of it.
git runs the filter when it writes one of those files, to put the dev changes in, and when it reads one, to take them out.
Only files that have dev changes go through it.
The new files are listed in the `info/exclude` of the repository, so git ignores them.
You never run the filter yourself.

```console
$ git config --get-regexp '^filter'
filter.octok-dev.clean sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f
filter.octok-dev.smudge sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' smudge %f
filter.octok-dev.required true
$ cat "$(git rev-parse --git-common-dir)/info/attributes"
# octok dev changes: begin
"main.py" filter=octok-dev
# octok dev changes: end
$ tail -3 "$(git rev-parse --git-common-dir)/info/exclude"
# octok dev changes: begin
/.env
# octok dev changes: end
```

### When you edit the same lines

A change to the very lines of a dev change cannot be told apart from it.
git then cannot take the dev changes out of that file, so it refuses to read it: `git status`, `git diff`, `git add` and `git commit` fail with the message below, and nothing is staged or committed.
It names the file and tells you what to do.
If the edit is a new dev change, as here, run `octok dev save` to make it the saved one.

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

The same message appears in an older checkout whose file no longer matches what is saved, because saving replaced the set for the whole repository.
Here the workspace `feat-cart` still has port 9005:

```console
$ cd "$OCTOTASK_HOME/workspaces/shop/feat-cart/shop-backend"
$ touch main.py
$ git status --short
octok: main.py changed where its dev changes are, so they cannot be taken out of it.
octok: if this is a dev change too, save it with `octok dev save`.
error: external filter 'sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f' failed 1
error: external filter 'sh '/Users/you/.octotask/dev/shop/shop-backend/filter.sh' clean %f' failed
fatal: main.py: clean filter 'octok-dev' failed
```

Run `octok dev save` in that checkout to make what it has the saved set, or edit the file to match.

## When they no longer apply

When a checkout starts from a commit that changed the same lines as a dev change, or that does not have the file, the dev changes of that file cannot be applied.
`octok branch`, `octok checkout` and `octok pr review` then create nothing and name the file.
`--allow-stale-dev` creates the workspace with that file as it is in git, and everything else as usual.
Make the dev changes again there, then run `octok dev save`.

A teammate pushes a branch that uses another port:

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

The workspace was created with `main.py` as git has it, and `.env` copied in.
After the edit and `octok dev save`, the saved set is the one that applies to the branch of your teammate.

Saving from a checkout with no dev changes left removes them:

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

## Where they are kept

Under the `octok` home of the machine where the repository lives, in `dev/<project>/<repo>`, so a `.env` never leaves that machine: see [Locations and SSH](locations.md).

| Path | Content |
|---|---|
| `dev/<project>/<repo>/dev/changed` | The tracked files with their dev changes. |
| `dev/<project>/<repo>/dev/new` | The files that git does not track. |
| `dev/<project>/<repo>/base` | The tracked files as git had them when they were saved. |
| `dev/<project>/<repo>/filter.sh` | The script of the git filter. |

A workspace with a [location of its own](locations.md#a-workspace-with-its-own-location), on a machine other than the one of the repository, uses the folder of its own machine instead: what `octok dev save` stores from it goes only to the workspaces later placed on that machine, and `octok dev show` does not show it.
They are deleted with the repository, with the project, and when the repository moves to another machine.
`octok dev show` reads them where they are, so it works from any folder, for the default project or the one named with `-p`.

## Limits

- A file that git ignores is saved only with `--file`, and a folder is not a file: name each file.
- The set is one per repository: two workspaces cannot have different dev changes at once for the same repository.
- Changes to the same lines as a dev change cannot be committed, as above.
- The secrets are not shown: [`octok dev show`](../reference/commands.md#octok-dev-show) and the JSON documents name the files that git does not track and never print them.
- It needs `git` and a POSIX shell on the machine of the repository.

See [JSON output](../agents/json-output.md) for the documents of `octok dev`.
