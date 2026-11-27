# Driving octok from an agent

How an AI coding agent uses `octok` for a task across several repositories: the skill, the rules of the interface and the documentation it reads.

## How to use it

1. Install `octok` on the machine of the agent: see [Installation](../getting-started/installation.md).
2. Install the [skill](#install-the-skill), or paste the [rules into `AGENTS.md`](#without-skills-agentsmd) for an agent without skills.
3. The agent passes `--json` to every command, checks the exit code, and decides on `error.code`: see [JSON output](json-output.md).
4. It reads the documentation of the installed version with `octok docs <topic>`.

## Install the skill

The skill tells an agent when to use `octok`, the usual commands, how to read their JSON, and what it must never do without asking the user.
It is in the [Agent Skills](https://agentskills.io) format, which Claude Code, Codex, Cursor and other agents load.
The plugin that carries it is the folder `plugins/octotask` of the repository, with the version of `octok` it describes.

Claude Code, version 2.1.275 or newer:

```sh
claude plugin install octotask --marketplace whiitex/octotask
```

Codex, version 0.146.0 or newer:

```sh
codex plugin marketplace add whiitex/octotask
codex plugin add octotask@octotask
```

Another agent: use the [`skills`](https://github.com/vercel-labs/skills) installer, or copy the folder `plugins/octotask/skills/octotask` of the repository into the skills folder of the agent.

```sh
npx skills add whiitex/octotask
```

### What the skill does

When a task touches more than one repository of a project, the agent:

1. Checks that `octok` is installed, and asks the user before installing it.
2. Reads the project with `octok status --json`.
   It never creates projects or registers repositories on its own: the user decides what a project contains.
3. Opens the workspace with `octok branch <branch> --json`, or `octok checkout <branch> --json` for an existing branch, such as a teammate's.
4. Works in each checkout it needs with `git`, and publishes with a plain `git push` in each one it changed.
5. Updates the workspace with `octok pull <branch> --json`, and removes it with `octok rm <branch> --json` once the work is merged.

It asks the user first for: `--force` on `octok rm` and `octok project rm`, `octok project rm` itself, `octok repo rm`, any change of a location, any command that changes how a project is set up (`project init`, `project use`, `project set`, `repo add`, `repo set`), `octok dev save`, `--allow-stale-dev` and `octok set-ssh-passphrase`.
An error message that suggests `--force` is a hint for the user, not for the agent.

## Without skills: AGENTS.md

Paste this into the `AGENTS.md` of the repository, or `CLAUDE.md`, or the file the agent reads:

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

## Rules

- **stdout for programs, stderr for people:** git progress, warnings and the `created workspace` lines go to stderr; do not parse it.
- **`--json` on every command,** before or after it: one document on one line of stdout, with `"schema": 1`.
  `octok up` prints one document per event.
- **Failures are documents too:** `{"schema":1,"error":{"code":...,"message":...}}` and an exit code other than 0, the same codes as without `--json`.
  Decide on `code`, which never changes for a kind of error, and see [the exit codes](json-output.md#exit-codes) for what to do.
- **Plain git underneath:** each checkout is a normal git working tree, and `octok` never pushes or opens a pull request.
- **No question is asked:** `octok` asks only when stdin is a terminal and `--json` is not given.
  A setting left out takes its default: `staging` and this machine for `octok project init`; no alias and the location and base branch of the project for `octok repo add`; the base branch for every repository that `octok pr review` does not name.
- **What would need an answer is refused:** a base branch not on the remote (`base_not_found`), `octok pr review` with no `--pr` and no `--branch` (`nothing_to_review`), and `octok set-ssh-passphrase` (`needs_terminal`).
- **`ssh` never asks either:** a passphrase or a host key to type fails at once with the error `ssh` (exit 7).
  The user then runs `octok set-ssh-passphrase <host>` once at a terminal.
- **`octok up` does not return while the repositories run:** start it in the background, read one document per line, and stop it with SIGINT, which stops every repository and exits with 130.

## Example: a failure and a conflict

A failure with no project, the progress on stderr that `2>/dev/null` hides, and two refusals:

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

## Reading the documentation

The pages describe the version of `octok` that prints them, so an agent prefers them to what it remembers.

| Source | What it gives |
|---|---|
| `octok docs` | The pages of the installed version, with topic and summary. No project and no network needed. |
| `octok docs <topic>` | One page as Markdown, also inside the `--json` document. The topic is the path without `.md`, such as `agents/json-output`. |
| `https://whiitex.github.io/octotask/llms.txt` | The index of the site for language models: every page with its link and summary. |
| `https://whiitex.github.io/octotask/llms-full.txt` | Every page in one file. |
| A page link plus `.md` | That page as Markdown, such as `https://whiitex.github.io/octotask/agents/json-output.md`. |

The site documents the latest release, and `octok docs` the installed version: prefer `octok docs` when they differ.

## Example: read a page

```console
$ octok docs agents/json-output | head -3
# JSON output

What each command of `octok` prints with `--json`, how a failure looks, and what the exit codes mean.
```

## Related pages

- [JSON output](json-output.md): every document, error code and exit code.
- [Built-in documentation](../commands/docs.md): `octok docs`.
- [Workspaces](../commands/workspaces.md): one workspace per feature, so several tasks run side by side without stashing or switching branches.
