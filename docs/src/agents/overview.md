# Driving octok from an agent

How an AI coding agent uses `octok` for a task that spans several repositories: installing the skill, the rules of the interface, and where the agent reads the documentation.

## Why octok suits agents

A coding agent with a shell can already run `git`, but a feature across a frontend, a backend and an agent is many commands, and each of them is a chance to leave one repository on the wrong branch.
`octok` gives the agent one command for the whole project and an interface made for programs:

- **One workspace per feature.**
  The agent works in a folder with a checkout of every repository, all on the same branch, and several tasks can run side by side without stashing or switching branches.
  See [Workspaces](../commands/workspaces.md).
- **JSON on every command.**
  `--json` prints one document on stdout, so the agent reads fields and not text.
  See [JSON output](json-output.md).
- **Exit codes that say why a command failed.**
  The agent knows whether to fix the command, check a name, ask the user, or go on.
- **Plain git underneath.**
  The checkouts are normal git working trees: the agent edits, tests and commits with the tools it knows, and `octok` never pushes or opens a pull request for it.
- **Nothing that waits for an answer.**
  With `--json`, `octok` never asks a question, so a command never hangs on a prompt.

## Install the skill

The skill tells an agent when to use `octok`, the usual sequence of commands, how to read their JSON output, and what it must never do without asking the user, such as `--force`.
It is in the [Agent Skills](https://agentskills.io) format, which Claude Code, Codex, Cursor and other agents load.
The folder `plugins/octotask` of the repository is the plugin that carries it, with the version of `octok` it describes.
The agent still needs `octok` itself: see [Installation](../getting-started/installation.md).

Install it in Claude Code, version 2.1.275 or newer:

```sh
claude plugin install octotask --marketplace whiitex/octotask
```

Install it in Codex, version 0.146.0 or newer:

```sh
codex plugin marketplace add whiitex/octotask
codex plugin add octotask@octotask
```

Install it for another agent with the [`skills`](https://github.com/vercel-labs/skills) installer, or copy the folder `plugins/octotask/skills/octotask` of the repository into the skills folder of the agent:

```sh
npx skills add whiitex/octotask
```

### What the skill does

When a task touches more than one repository of a project, the agent loads the skill and follows it:

1. It checks that `octok` is installed, and asks the user before installing it.
2. It reads the project with `octok status --json`.
   It does not create projects or register repositories on its own: the user decides what a project contains.
3. It opens the workspace of the branch with `octok branch <branch> --json`, or with `octok checkout <branch> --json` for a branch that exists, such as a teammate's.
4. It works in each checkout with `git`, touching only the repositories the task needs, and publishes with a plain `git push` in each checkout it changed.
5. It updates the workspace with `octok pull <branch> --json`, and removes it with `octok rm <branch> --json` when the work is merged.

The skill also lists what the agent asks the user before it does it: `--force` on `octok rm` and `octok project rm`, `octok project rm` itself, `octok repo rm`, any change of a location, any command that changes how a project is set up (`project init`, `project use`, `project set`, `repo add`, `repo set`), `octok dev save`, `--allow-stale-dev` and `octok set-ssh-passphrase`.
An error message may suggest `--force`: that is a hint for the user, not for the agent.

## Without skills: AGENTS.md

An agent that does not load skills can read the rules from the instructions file of the repository.
Paste this into its `AGENTS.md` (or `CLAUDE.md`, or the file the agent reads):

```markdown
## Working across repositories with octok

This project's repositories are managed with `octok`: one workspace per feature, with a checkout of every repository on the same branch.
Use it when a task touches more than one repository.

- Always pass `--json`. Read stdout, never stderr. Check the exit code first.
- On a failure stdout holds `{"schema":1,"error":{"code":...,"message":...}}`: decide on `error.code`.
- See the project with `octok status --json`.
- New branch: `octok branch <branch> --json`. Existing branch, such as a teammate's: `octok checkout <branch> --json`.
  Both print where each repository is checked out: work in those paths with plain git.
- Publish with `git push` in each checkout you changed. `octok` never pushes.
- Update with `octok pull <branch> --json`, remove with `octok rm <branch> --json`.
- A workspace is named after its branch with `/` turned into `-`, and commands take the branch.
- Never without asking the user: `--force`, `octok project rm`, `octok repo rm`, changing a `--location`, `octok dev save`, `--allow-stale-dev`, creating or changing projects and repositories.
- The documentation of the installed version: `octok docs` lists the pages, `octok docs <topic>` prints one.
```

## The rules of the interface

- **stdout is for programs, stderr for people.**
  A command prints its result on stdout.
  The progress of git, the warnings and the `created workspace` lines go to stderr.
  Do not parse stderr.
- **`--json` on every command, before or after it.**
  The command prints one document on one line of stdout, with `"schema": 1`, and nothing else on stdout.
  `octok up` prints one document per event.
- **Failures are documents too.**
  A failed command prints `{"schema":1,"error":{"code":...,"message":...}}` and exits with a code other than 0.
  The `code` never changes for a kind of error, so decide on it and not on the message.
- **Exit codes say what kind of failure it was.**
  The same codes apply with and without `--json`.
  The full table, with what to do for each, is in [JSON output](json-output.md#exit-codes):

  | Code | Meaning |
  |---|---|
  | 0 | Done. |
  | 2 | The command is not written the way `octok` takes it. Fix it. |
  | 3 | Something it names is not there. Check the name. |
  | 4 | Refused, because it would lose work. Commit and push, or ask the user. |
  | 5 | Refused, because it conflicts with what exists. Read `error.code`. |
  | 6 | At least one repository failed. Read `repos`. |
  | 7 | Another machine or a program cannot be reached. Ask the user. |
  | 1, 130 | Another error, or `octok up` was stopped. |

Here is the interface at work: a failure with no project, the progress on stderr that `2>/dev/null` hides, and a conflict.

```console
$ octok status --json || echo "exit $?"
{"schema":1,"error":{"code":"no_default_project","message":"no project given and no default project set (use `octok project use <name>`)"}}
exit 2
$ octok project init shop --json
{"schema":1,"project":"shop","base":"staging","location":"local","file":"/Users/you/.octotask/projects/shop.yaml"}
$ octok repo add git@github.com:acme/shop-frontend.git --alias frontend --json
{"schema":1,"project":"shop","repo":{"key":"shop-frontend","alias":"frontend","url":"git@github.com:acme/shop-frontend.git","base":"staging","location":"local"}}
$ octok repo add git@github.com:acme/shop-backend.git --alias backend --json
{"schema":1,"project":"shop","repo":{"key":"shop-backend","alias":"backend","url":"git@github.com:acme/shop-backend.git","base":"staging","location":"local"}}
$ octok branch feat/cart --json 2>/dev/null
{"schema":1,"project":"shop","workspace":"feat-cart","path":"/Users/you/.octotask/workspaces/shop/feat-cart","repos":[{"key":"shop-frontend","path":"/Users/you/.octotask/workspaces/shop/feat-cart/shop-frontend","outcome":"created"},{"key":"shop-backend","path":"/Users/you/.octotask/workspaces/shop/feat-cart/shop-backend","outcome":"created"}]}
$ octok branch feat/cart --json || echo "exit $?"
{"schema":1,"error":{"code":"workspace_exists","message":"workspace `feat-cart` already exists at /Users/you/.octotask/workspaces/shop/feat-cart","workspace":"feat-cart","path":"/Users/you/.octotask/workspaces/shop/feat-cart"}}
exit 5
$ octok rm feat/nope --json || echo "exit $?"
{"schema":1,"error":{"code":"workspace_not_found","message":"no workspace for `feat/nope`: create it with `octok checkout <branch>`, or with `octok checkout -b <branch>` for a new branch","workspace":"feat-nope"}}
exit 3
```

### What changes without a terminal

An agent has no terminal, and `octok` does not guess what a person would have typed:

- **No question is asked.**
  `octok` asks only when its stdin is a terminal and `--json` is not given, so an agent never sees a question.
  A setting left out takes its default: `staging` and this machine for `octok project init`; no alias, and the location and the base branch of the project, for `octok repo add`; the base branch for every repository that `octok pr review` does not name.
- **What would need an answer is refused or assumed.**
  A base branch that is not on the remote is refused (`base_not_found`) instead of confirmed.
  `octok pr review` with no `--pr` and no `--branch` is refused (`nothing_to_review`), because the agent has not said what to review.
  `octok set-ssh-passphrase` is refused (`needs_terminal`), because it only works by asking.
- **`ssh` never asks either.**
  It runs with `BatchMode=yes`: a passphrase or a host key that would have to be typed fails at once with the error `ssh` (exit 7), and the user runs `octok set-ssh-passphrase <host>` once at a terminal.
- **Secrets stay out of the output.**
  The files that git does not track are only named, and the documents about a repository leave out its `run` and `setup` commands.
  `octok up` is the one command that shows them, in the lines it prints before running each one.
- **`octok up` does not return while the repositories run.**
  The agent starts it in the background, reads one JSON document per line, and stops it with SIGINT, which stops every repository and exits with 130.

## How an agent reads the documentation

The documentation is part of the interface: the pages describe the version of `octok` that prints them, so an agent should prefer them to what it remembers.

- **`octok docs`** lists the pages of the installed version with their topic and a one-line summary, and `octok docs <topic>` prints one page as Markdown.
  It needs no project and no network.
  The topic is the path of the page without `.md`, for example `commands/workspaces` or `agents/json-output`.
  Reading a page is the one exception to `--json`: the page is the same Markdown either way.
- **`https://whiitex.github.io/octotask/llms.txt`** is the index of the site for language models: every page with its link and its summary.
- **`https://whiitex.github.io/octotask/llms-full.txt`** is every page in one file.
- **Every page of the site as Markdown**: add `.md` to the path of its link, for example `https://whiitex.github.io/octotask/agents/json-output.md`.

The site documents the latest release.
`octok docs` documents the version installed, so it is the one to use when the two differ.
See [Built-in documentation](../commands/docs.md).

```console
$ octok docs agents/json-output | head -3
# JSON output

What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.
```
