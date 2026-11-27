# JSON output

What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.

Every command of `octok` accepts `--json`, before or after the command.

## Rules

- **One document:** one JSON document on one line of stdout, and nothing else on stdout.
  `octok up` prints one document per event, one per line.
- **`schema`:** every document carries `"schema": 1`.
  A new field can appear in any release; a field is removed or changes meaning only with a higher `schema`.
- **stderr is for people:** it carries git progress and warnings, so do not parse it.
- **No questions:** `octok` never asks, even at a terminal, and `ssh` runs with `BatchMode=yes`.
  A setting left out takes its default.
- **Failures are documents too:** an `error` document on stdout, and an exit code other than 0.
- **No secrets:** untracked files are only named, and no document about a repository holds its `run` or `setup` command.
  `octok up` shows them only in the lines it prints before running each one.
- **`--help`, `--version` and `octok help`** print text; `octok help` does not take `--json`.

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

The same with and without `--json`.

| Code | Meaning | What to do |
|---|---|---|
| 0 | Done. | |
| 1 | Another error: git or `gh` failed, a file cannot be read. | Read `error.message`. |
| 2 | The command is not written the way `octok` takes it: an unknown option, an invalid name, no project given. | Fix the command. |
| 3 | Something it names is not there: a project, repository, branch, workspace, run command or page of the documentation. | Check the name with `octok status --json`, or the page with `octok docs --json`. |
| 4 | Refused, because it would lose work that exists nowhere else. | Commit and push, or ask the user. Never add `--force` on your own. |
| 5 | Refused, because it conflicts with what exists: already there, in use, or no longer applies. | Read `error.code`. |
| 6 | At least one repository failed, in a command that goes through them one by one. The document is the normal one, with an `error` in each failed entry. | Read `repos`. |
| 7 | Another machine or a program cannot be reached: `ssh` failed, or `git`, `gh`, `ssh`, `sh` or another program `octok` runs is not installed. | Ask the user. |
| 130 | `octok up` was stopped with Ctrl-C or a signal. | |

- Exit 6 has no top-level `error`, even when every repository failed, whatever their errors are.
- `octok up` exits with 1 when a repository failed and with 130 when stopped: its last documents are then `exited` events, not an `error`.

## Failures

```json
{"schema":1,"error":{"code":"workspace_has_work","message":"workspace `feat-login` has work that would be lost:\n  backend: 1 unpushed commit on `feat/login`\nremove it anyway with `octok rm --force <branch>`","workspace":"feat-login","reasons":["backend: 1 unpushed commit on `feat/login`"]}}
```

- **`code`** never changes for a kind of error: decide on it, not on the message.
- **`message`** is text for a person, and can change in any release.
- **Other fields** say what the error is about, as listed below.
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

| Field | Meaning |
|---|---|
| `repos[].cached` | Whether the repository is cloned. |
| `repos[].commit` | The tip of the base branch at the last fetch, or `null`. |
| `workspaces[].branch` | `null` when no checkout has a branch. |
| `checkouts[].repo` | The key of the repository. A repository without a checkout in the workspace has no entry. |
| `checkouts[].branch` | `null` on a detached HEAD. |
| `checkouts[].on_remote` | Whether the remote had the branch at the last fetch. |
| `checkouts[].commit` | `null` when the checkout cannot be read. |
| `checkouts[].error` | Then why, as an `error` of [Failures](#failures); `null` otherwise. |
| `orphan_caches` | The clones of repositories no longer in the project. |

### `octok branch`, `octok checkout`, `octok pr review`

```json
{"schema":1,"project":"shop","workspace":"feat-login","path":"/Users/you/.octotask/workspaces/shop/feat-login","repos":[{"key":"shop-frontend","path":"/Users/you/.octotask/workspaces/shop/feat-login/shop-frontend","outcome":"created"},{"key":"shop-backend","path":"api-dev-1:/home/ubuntu/.octotask/workspaces/shop/feat-login/shop-backend","outcome":"tracked"}]}
```

`path` is the folder of the workspace, on this machine when it has one here, and `repos[].path` is each checkout.

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
`octok project rm` has no `project`, and adds `default`: the default project left, or `null`.

```json
{"schema":1,"removed":"shop","default":null}
```

### `octok fetch`, `octok pull`, `octok open`

```json
{"schema":1,"project":"shop","workspace":"feat-login","repos":[{"key":"shop-frontend","outcome":"updated"},{"key":"shop-backend","error":{"code":"ssh","message":"cannot run commands on `api-dev-1` over ssh","host":"api-dev-1","stderr":""}}]}
```

- `workspace` is there for `octok pull` and `octok open` only.
- Each entry of `repos` has an `outcome`, or an `error` as in [Failures](#failures) when it failed; then the command exits with 6.

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
| `open` | `{"path":"<folder>"}` | The editor was started: `path` is the folder, written `<host>:<path>` on another machine. |

`octok open` opens nothing and prints a failure, with no `repos`, for: a missing workspace (`workspace_not_found`, 3), a chosen repository not registered (`repo_not_found`, 3) or without a checkout (`no_worktree`, 3), no workspace told from the current folder (`not_in_workspace`, 2), an editor not installed (`tool_not_found`, 7), a machine that does not answer over ssh (`ssh`, 7, before any window opens).

### `octok project init`, `octok project set`

```json
{"schema":1,"project":"shop","base":"staging","location":"local","file":"/Users/you/.octotask/projects/shop.yaml"}
```

- `file` holds the settings of the project.
- `octok project init` also makes the project the default one.
- `octok project set --base <branch>` warns on stderr when the remote of a repository lacks that branch.
- `octok project set --location <location>` and `octok project set --no-location` add `repos`: the repositories cloned again where the project now lives, as in [`octok fetch`](#octok-fetch-octok-pull-octok-open), and exit with 6 when one failed.

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
- `octok repo set --branch <branch>` adds `workspace`, the workspace whose settings changed.
- The run and setup commands are not in the document, so a change to them does not show.

### `octok repo ls`

```json
{"schema":1,"project":"shop","repos":[{"key":"shop-backend","alias":"backend","url":"git@github.com:acme/shop-backend.git","base":"staging","location":"local"}]}
```

### `octok dev save`, `octok dev show`

```json
{"schema":1,"project":"shop","repos":[{"key":"shop-backend","alias":"backend","changed":["config/dev.py"],"new":[".env"],"diff":"diff --git a/config/dev.py b/config/dev.py\n..."}]}
```

`changed` lists the tracked files with dev changes, and `diff` holds their changes.
`new` lists the files that git does not track, whose content is never printed.

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

- **`code`** of `exited` is the exit code of the command; a command a signal ended has 128 plus the signal number, 143 after a stop.
- **`signal`** is `null`, except when the shell running the commands was killed itself: `code` is then `null`, and `signal` is 9 for commands still running 10 seconds after a stop.
- **A failed setup** ends its repository: `exited` has its code, and the run command does not start.
- **A failure before anything starts** prints one failure document instead.

### `octok docs`

```json
{"schema":1,"topics":[{"topic":"agents/json-output","title":"JSON output","summary":"What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean."},{"topic":"reference/commands","title":"Command reference","summary":"Every command of `octok` with its arguments and options, as its `--help` describes them."}]}
```

`topics` lists every page of the documentation of this version, in reading order (two shown here).
`topic` is what `octok docs <topic>` takes, `title` the title of the page and `summary` the sentence under it.

`octok docs <topic>` prints one page, with its Markdown in `markdown`:

```json
{"schema":1,"topic":"agents/json-output","title":"JSON output","summary":"What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.","markdown":"# JSON output\n\nWhat each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.\n..."}
```

`octok docs` needs no project, and a topic that is not a page fails with `topic_not_found`.
