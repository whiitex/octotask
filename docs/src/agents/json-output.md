# JSON output

What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.

Every command of `octok` accepts `--json`, before or after the command.

## Rules

- **One document.**
  With `--json`, a command prints one JSON document on one line of stdout, and nothing else on stdout.
  `octok up` is the exception: it prints one document per event, each on its own line.
- **`schema`.**
  Every document carries `"schema": 1`.
  A new field can appear in any release.
  A field is removed, or changes its meaning, only together with a higher `schema`.
- **stderr is for people.**
  It carries the progress of git and the warnings.
  Do not parse it.
- **No questions.**
  With `--json`, `octok` never asks anything, even at a terminal, and `ssh` runs with `BatchMode=yes`.
  A setting left out takes its default.
- **Failures are documents too.**
  A command that fails prints a document with an `error` on stdout and exits with a code other than 0.
- **No secrets.**
  The files that git does not track are only named.
  The documents about a repository leave out its `run` and `setup` commands.
  `octok up` is the one command that shows them, in the lines it prints before running each one.
- **`--help`, `--version` and `octok help`** print text.
  `octok help` does not take `--json`.

## Names used in every document

| Name | Meaning |
|---|---|
| `project` | The name of the project. |
| `workspace` | The name of the workspace: its branch with every `/` turned into `-`. `feat/login` gives `feat-login`. |
| `repos` | A list with one entry per repository, in the order of the project. |
| `key` | The key of a repository, in an entry of `repos`. |
| `path` | A folder. On another machine it is written `<host>:<path>`, for example `api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/backend`. |
| `location` | Where a repository lives: `local`, or `<host>:<path>` on another machine, with the folder that `octok` uses there. |

## Exit codes

They are the same with and without `--json`.

| Code | Meaning | What to do |
|---|---|---|
| 0 | The command did what it was asked. | |
| 1 | Another error: git or `gh` failed, a file cannot be read. | Read `error.message`. |
| 2 | The command is not written the way `octok` takes it: an unknown option, a name that is not valid, no project given. | Fix the command. |
| 3 | Something the command names is not there: a project, a repository, a branch, a workspace, a run command, a page of the documentation. | Check the name with `octok status --json`, or the page with `octok docs --json`. |
| 4 | Refused, because it would lose work that exists nowhere else. | Commit and push, or ask the user. Never add `--force` on your own. |
| 5 | Refused, because it conflicts with what exists: already there, in use, or no longer applies. | Read `error.code`. |
| 6 | The command went through the repositories one by one, and at least one failed. The document is the normal one, with an `error` in the entry of each failed repository. | Read `repos`. |
| 7 | Another machine or a program cannot be reached: `ssh` failed, or a program that `octok` runs, such as `git`, `gh`, `ssh` or `sh`, is not installed. | Ask the user. |
| 130 | `octok up` was stopped with Ctrl-C or a signal. | |

- A command that exits with 6 has no `error` at the top of its document, even when every repository failed.
  The exit code is 6 whatever the errors of the repositories are.
- `octok up` exits with 1 when a repository ended with a failure, and with 130 when it was stopped.
  In both cases the last documents are the `exited` events, not an `error`.

## Failures

```json
{"schema":1,"error":{"code":"workspace_has_work","message":"workspace `feat-login` has work that would be lost:\n  backend: 1 unpushed commit on `feat/login`\nremove it anyway with `octok rm --force <branch>`","workspace":"feat-login","reasons":["backend: 1 unpushed commit on `feat/login`"]}}
```

- `code` never changes for a kind of error.
  Decide on it, not on the message.
- `message` is the text a person would read.
  It can change at any release.
- The other fields are what the error is about.
  They are listed below.
  `reasons` and `reviewed` hold text for people, one line per repository.
  `exit_code` is `null` for a command that a signal ended.

| `code` | Exit | Other fields |
|---|---|---|
| `usage` | 2 | |
| `invalid_name` | 2 | `kind`, `value`, `reason` |
| `invalid_location` | 2 | `value`, `reason` |
| `invalid_env` | 2 | `repo`, `name`, `reason` |
| `env_cycle` | 2 | `chain` |
| `invalid_url` | 2 | `url` |
| `no_repo_key` | 2 | `url` |
| `no_default_project` | 2 | |
| `nothing_to_review` | 2 | |
| `repo_chosen_twice` | 2 | `repo` |
| `not_in_workspace` | 2 | `usage` |
| `needs_terminal` | 2 | `command` |
| `repo_not_found` | 3 | `repo` |
| `project_not_found` | 3 | `project` |
| `no_repos` | 3 | `project` |
| `base_not_found` | 3 | `repo`, `base` |
| `branch_not_found` | 3 | `branch` |
| `branch_not_on_remote` | 3 | `repo`, `branch` |
| `workspace_not_found` | 3 | `workspace` |
| `nothing_to_run` | 3 | `workspace` |
| `no_run_command` | 3 | `repo` |
| `no_worktree` | 3 | `repo`, `workspace` |
| `undefined_variable` | 3 | `repo`, `name`, `variable` |
| `dev_file_not_found` | 3 | `file` |
| `topic_not_found` | 3 | `topic` |
| `workspace_has_work` | 4 | `workspace`, `reasons` |
| `duplicate_repo` | 5 | `repo`, `url` |
| `duplicate_alias` | 5 | `alias`, `repo` |
| `project_exists` | 5 | `project` |
| `repo_in_use` | 5 | `repo`, `workspaces` |
| `branch_exists` | 5 | `branch`, `repos` |
| `branch_in_use` | 5 | `repo`, `branch`, `path` |
| `project_in_use` | 5 | `project`, `workspaces` |
| `workspace_of_another_branch` | 5 | `workspace`, `branch` |
| `reviews_other_branches` | 5 | `workspace`, `reviewed` |
| `workspace_exists` | 5 | `workspace`, `path` |
| `stale_dev_changes` | 5 | `repo`, `files` |
| `tool_not_found` | 7 | `tool` |
| `spawn` | 7 | `program`, `dir` |
| `ssh` | 7 | `host`, `stderr` |
| `unsupported_version` | 1 | `path`, `version`, `expected` |
| `name_mismatch` | 1 | `path`, `name`, `expected` |
| `parse` | 1 | `path` |
| `serialize` | 1 | `path` |
| `io` | 1 | `path` |
| `fork_pull_request` | 1 | `repo`, `number` |
| `no_home` | 1 | |
| `unexpected_output` | 1 | `command`, `output` |
| `command_failed` | 1 | `command`, `dir`, `exit_code`, `stderr` |

`set-ssh-passphrase` always fails with `needs_terminal` under `--json`: `ssh-add` asks for the passphrase at the terminal.

## Documents

### `octok status`

```json
{"schema":1,"project":"shop","repos":[{"key":"shop-backend","alias":"backend","url":"git@github.com:acme/shop-backend.git","base":"staging","location":"local","cached":true,"commit":{"sha":"ce09b94","subject":"Add the cart","date":"2026-11-19T18:01:05+01:00","age":"2 hours ago"}}],"workspaces":[{"name":"feat-login","branch":"feat/login","path":"/Users/you/.octotask/workspaces/shop/feat-login","checkouts":[{"repo":"shop-backend","path":"/Users/you/.octotask/workspaces/shop/feat-login/shop-backend","branch":"feat/login","on_remote":false,"commit":{"sha":"ce09b94","subject":"Add the cart","date":"2026-11-19T18:01:05+01:00","age":"2 hours ago"},"error":null}]}],"orphan_caches":[]}
```

- `repos[].cached` says whether the repository is cloned.
  `repos[].commit` is the tip of the base branch at the last fetch, or `null`.
- `workspaces[].branch` is `null` when no checkout has a branch.
- `workspaces[].checkouts[].repo` is the key of the repository.
  A repository without a checkout in the workspace has no entry.
- `checkouts[].branch` is `null` on a detached HEAD.
- `checkouts[].on_remote` says whether the remote had the branch at the last fetch.
- `checkouts[].commit` is `null` when the checkout cannot be read.
  `checkouts[].error` then says why, as the `error` described under [Failures](#failures). It is `null` otherwise.
- `orphan_caches` lists the clones of repositories that are no longer in the project.

### `octok branch`, `octok checkout`, `octok pr review`

```json
{"schema":1,"project":"shop","workspace":"feat-login","path":"/Users/you/.octotask/workspaces/shop/feat-login","repos":[{"key":"shop-frontend","path":"/Users/you/.octotask/workspaces/shop/feat-login/shop-frontend","outcome":"created"},{"key":"shop-backend","path":"api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend","outcome":"tracked"}]}
```

- `path` is the folder of the workspace: the one on this machine when it has one here.
- `repos[].path` is the checkout of the repository.
- `repos[].outcome` says what the command did for the repository:

| `outcome` | Meaning |
|---|---|
| `created` | A new branch from the base branch. |
| `tracked` | The branch that the remote already had. |
| `restored` | The branch was still in the clone, without a checkout. |
| `kept` | The checkout was already there. |

### `octok rm`, `octok repo rm`, `octok project rm`

```json
{"schema":1,"project":"shop","removed":"feat-login"}
```

`removed` is the name of the workspace, the key of the repository or the name of the project.
`octok project rm` has no `project`, and adds `default`: the default project that is left, or `null`.

```json
{"schema":1,"removed":"shop","default":null}
```

### `octok fetch`, `octok pull`

```json
{"schema":1,"project":"shop","workspace":"feat-login","repos":[{"key":"shop-frontend","outcome":"updated"},{"key":"shop-backend","error":{"code":"ssh","message":"cannot run commands on `api-dev-1` over ssh","host":"api-dev-1","stderr":""}}]}
```

- `workspace` is there for `octok pull` only.
- An entry of `repos` has an `outcome` when the repository went well, and an `error` when it failed.
  The `error` is the one described under [Failures](#failures).
- The command exits with 6 when at least one repository failed.

| Command | `outcome` | Meaning |
|---|---|---|
| `fetch` | `cloned` | The repository was not cloned yet. |
| `fetch` | `fetched` | |
| `pull` | `updated` | The checkout was fast-forwarded. |
| `pull` | `up_to_date` | |
| `pull` | `no_remote_branch` | Skipped: the branch is not on the remote yet. |
| `pull` | `diverged` | Skipped: the branch and the remote one both have commits the other lacks. |
| `pull` | `detached` | Skipped: detached HEAD. |
| `pull` | `no_worktree` | Skipped: the repository has no checkout in this workspace. |

### `octok project init`, `octok project set`

```json
{"schema":1,"project":"shop","base":"staging","location":"local","file":"/Users/you/.octotask/projects/shop.yaml"}
```

`file` is the file that holds the settings of the project.
`octok project init` also makes the project the default one.
`octok project set --base <branch>` warns on stderr when the remote of a repository lacks that branch.
`octok project set --location <location>` and `octok project set --no-location` add `repos`: the repositories cloned again where the project now lives, as in [`octok fetch`](#octok-fetch-octok-pull).
The command then exits with 6 when one of them failed.

### `octok project use`

```json
{"schema":1,"default":"shop"}
```

### `octok project ls`

```json
{"schema":1,"default":"shop","projects":["blog","shop"]}
```

`default` is `null` when no project is the default one.

### `octok repo add`, `octok repo set`

```json
{"schema":1,"project":"shop","repo":{"key":"shop-backend","alias":"backend","url":"git@github.com:acme/shop-backend.git","base":"staging","location":"local"}}
```

- `alias` is `null` for a repository without one.
- `base` is the base branch in effect: the one of the repository, else the one of the project.
- `octok repo set --branch <branch>` adds `workspace`, the workspace whose run command changed.
- The run and setup commands are not in the document, so it does not show a change to them.

### `octok repo ls`

```json
{"schema":1,"project":"shop","repos":[{"key":"shop-backend","alias":"backend","url":"git@github.com:acme/shop-backend.git","base":"staging","location":"local"}]}
```

### `octok dev save`, `octok dev show`

```json
{"schema":1,"project":"shop","repos":[{"key":"shop-backend","alias":"backend","changed":["config/dev.py"],"new":[".env"],"diff":"diff --git a/config/dev.py b/config/dev.py\n..."}]}
```

- `changed` lists the tracked files with dev changes, and `diff` is their changes.
- `new` lists the files that git does not track.
  Their content is never printed.

### `octok up`

One document per line, until every repository has ended.

```json
{"schema":1,"event":"started","project":"shop","workspace":"feat-login","repos":["shop-frontend","shop-backend"]}
{"schema":1,"event":"output","repo":"shop-backend","line":"$ PORT=9005 uv run main.py"}
{"schema":1,"event":"output","repo":"shop-backend","line":"Uvicorn running on http://0.0.0.0:9005"}
{"schema":1,"event":"exited","repo":"shop-backend","code":0,"signal":null}
```

| `event` | Meaning |
|---|---|
| `started` | The first line. `repos` are the keys of the repositories that were started. |
| `output` | One line that the repository printed, on stdout or stderr. A `line` that starts with `$ ` is a command that `octok` runs. |
| `exited` | The commands of the repository ended. |

In an `exited` event, `code` is the exit code of the command.
A command that a signal ended has the code a shell gives it, 128 plus the number of the signal: 143 after a stop.
`signal` is `null`, except when the shell that runs the commands was itself killed: `code` is then `null`.
That happens to commands that are still running 10 seconds after a stop, with `signal` 9.

A command that fails before anything starts prints one failure document instead.
A setup command that fails ends its repository: the `exited` event has its code, and the run command does not start.

### `octok docs`

```json
{"schema":1,"topics":[{"topic":"agents/json-output","title":"JSON output","summary":"What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean."},{"topic":"reference/commands","title":"Command reference","summary":"Every command of `octok` with its arguments and options, as its `--help` describes them."}]}
```

`topics` lists every page of the documentation of this version of `octok`, in reading order.
`topic` is what `octok docs <topic>` takes, `title` is the title of the page and `summary` is the sentence under its title.
The example above shows two pages of the list.

`octok docs <topic>` prints one page, with its text as Markdown in `markdown`:

```json
{"schema":1,"topic":"agents/json-output","title":"JSON output","summary":"What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.","markdown":"# JSON output\n\nWhat each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.\n..."}
```

`octok docs` needs no project.
A topic that is not a page fails with `topic_not_found`.
