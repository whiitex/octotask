# Built-in documentation

How `octok docs` prints the pages of this documentation from the binary you run, and where else the same pages are.

## What it is for

The documentation of `octok` is a set of pages, one for each subject: a command, a concept, a guide.
`octok docs` prints them in your terminal, so you and your coding agents can read the pages that describe the version of `octok` you run, with no network.
It needs no project and no home folder: everything it prints is built into the binary.

```text
octok docs [<topic>]
```

## The list of topics

A **topic** is the path of a page, without `.md`: `commands/project`, `concepts`, `reference/commands`.
Without a topic, `octok docs` lists every page, one line each, in the order of the site: the topic, the title and a one-sentence summary.

```console
$ octok docs | head -3
introduction                  Introduction: The CLI `octok` runs the same git operation on every repository of a project, so a feature that touches a frontend, a backend and an agent is one branch, one folder and one command.
getting-started/installation  Installation: What `octok` needs on your machine, how to install it from a release or from source, and how to update, remove and relocate it.
getting-started/quick-start   Quick start: A first session with `octok`: one project with two repositories, a workspace on a new branch, a commit pushed from it, a sync and a clean removal.
$ octok docs | grep '^commands/docs '
commands/docs                 Built-in documentation: How `octok docs` prints the pages of this documentation from the binary you run, and where else the same pages are.
```

The summary is the sentence under the title of the page.

## One page

With a topic, `octok docs` prints that page as Markdown.

```console
$ octok docs commands/docs | head -3
# Built-in documentation

How `octok docs` prints the pages of this documentation from the binary you run, and where else the same pages are.
$ octok docs reference/commands | head -3
# Command reference

Every command of `octok` with its arguments and options, as its `--help` describes them.
```

The page is the one of the site for the same version: the files are embedded in the binary when it is built.
Only `reference/commands` is not a file: it is made from the command line of `octok` itself, so it always matches `octok --help`.

## `--json`

With `--json`, the list is one document with the topics, and a page is one document with its topic, title, summary and the Markdown text:

```console
$ octok docs --json | head -c 80
{"schema":1,"topics":[{"topic":"introduction","title":"Introduction","summary":"
$ octok docs commands/docs --json | head -c 140
{"schema":1,"topic":"commands/docs","title":"Built-in documentation","summary":"How `octok docs` prints the pages of this documentation from
```

See [JSON output](../agents/json-output.md) for the exact shapes.

## An unknown topic

A topic that is not in the list is refused, and nothing is printed to stdout:

```console
$ octok docs commands/nothing
error: topic `commands/nothing` not found: list the topics with `octok docs`
$ octok docs commands/nothing --json
{"schema":1,"error":{"code":"topic_not_found","message":"topic `commands/nothing` not found: list the topics with `octok docs`","topic":"commands/nothing"}}
```

The exit code is 3, because the topic is something the command names that is not there.

## The same pages elsewhere

- **The site**, `https://whiitex.github.io/octotask/`, with search and a page for each topic.
- **Each page as Markdown**, at the address of the page with `.md` in place of `.html`, for example `https://whiitex.github.io/octotask/commands/docs.md`.
- **`llms.txt`**, at `https://whiitex.github.io/octotask/llms.txt`: the list of the pages with their summary, for a language model.
- **`llms-full.txt`**, at `https://whiitex.github.io/octotask/llms-full.txt`: every page in one file.

The site is rebuilt at every release, so it follows the latest one, and `octok docs` follows the binary in your hands: when they differ, trust `octok docs`.
The skill that teaches an agent to drive `octok` points to `octok docs`: see [Driving octok from an agent](../agents/overview.md).
