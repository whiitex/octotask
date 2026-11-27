# Built-in documentation

How `octok docs` prints the pages of this documentation from the binary you run, and where else the same pages are.

## How to use it

1. List the pages with `octok docs`.
2. Print one with `octok docs <topic>`, such as `octok docs commands/project`.

## Options

| Command | What it does |
|---|---|
| `octok docs` | Lists every page in the order of the site: topic, title and summary, the sentence under the title. |
| `octok docs <topic>` | Prints that page as Markdown. |
| `... --json` | Prints the list, or the page with its Markdown, as one document: see [JSON output](../agents/json-output.md). |

## Rules

- **Topics:** a topic is the path of a page without `.md`, such as `concepts` or `reference/commands`.
- **Built in:** the pages are embedded in the binary, so they match the version you run and need no network, project or home folder.
- **`reference/commands`** is made from the command line of `octok`, so it always matches `octok --help`.
- **An unknown topic** exits with code 3 and prints nothing to stdout.
- **Online:** the site `https://whiitex.github.io/octotask/` has search, and each page as Markdown with `.md` in place of `.html`.
  For a language model, `llms.txt` lists the pages with their summary, and `llms-full.txt` holds every page in one file, both at the root of the site.
- **Which to trust:** the site follows the latest release, and `octok docs` the binary you run: when they differ, trust `octok docs`.
  The [agent skill](../agents/overview.md) points to `octok docs`.

## Example: the list

```console
$ octok docs | head -3
introduction                  Introduction: The CLI `octok` runs the same git operation on every repository of a project, so a feature that touches a frontend, a backend and an agent is one branch, one folder and one command.
getting-started/installation  Installation: What `octok` needs on your machine, how to install it from a release or from source, and how to update, remove and relocate it.
getting-started/quick-start   Quick start: A first session with `octok`: one project with two repositories, a workspace on a new branch, a commit pushed from it, a sync and a clean removal.
$ octok docs | grep '^commands/docs '
commands/docs                 Built-in documentation: How `octok docs` prints the pages of this documentation from the binary you run, and where else the same pages are.
```

## Example: one page

```console
$ octok docs commands/docs | head -3
# Built-in documentation

How `octok docs` prints the pages of this documentation from the binary you run, and where else the same pages are.
$ octok docs reference/commands | head -3
# Command reference

Every command of `octok` with its arguments and options, as its `--help` describes them.
```

## Example: JSON

```console
$ octok docs --json | head -c 80
{"schema":1,"topics":[{"topic":"introduction","title":"Introduction","summary":"
$ octok docs commands/docs --json | head -c 140
{"schema":1,"topic":"commands/docs","title":"Built-in documentation","summary":"How `octok docs` prints the pages of this documentation from
```

## Example: an unknown topic

```console
$ octok docs commands/nothing
error: topic `commands/nothing` not found: list the topics with `octok docs`
$ octok docs commands/nothing --json
{"schema":1,"error":{"code":"topic_not_found","message":"topic `commands/nothing` not found: list the topics with `octok docs`","topic":"commands/nothing"}}
```

## Related pages

- [Command reference](../reference/commands.md), the page made from `octok --help`.
- [Driving octok from an agent](../agents/overview.md).
- [JSON output](../agents/json-output.md), for the shapes of `--json`.
