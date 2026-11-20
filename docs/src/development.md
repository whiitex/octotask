# Development

How to build, test and release `octok`, how the workspace is laid out, and how these pages are written, checked and published.

## Building and testing

You need [Rust](https://rustup.rs) 1.89 or newer and `git`.
The tests need `git` and `gh`.
A fake `ssh` stands in for the other machine and a fake `gh` for GitHub in many of them, and the git remotes are local repositories, so the tests use no network.
If `cargo` is not on `PATH`, run `. "$HOME/.cargo/env"` first.

| Command | What it does |
|---|---|
| `make format` | Formats the code. |
| `make test` | Runs the tests of the whole workspace. |
| `make build` | Builds the release binary. |
| `make install` | Installs `octok` into `~/.cargo/bin`, with `cargo install --path crates/octotask-cli --locked`. |
| `make docs` | Builds the documentation site into `target/book`. Needs `mdbook` on `PATH`. |
| `make docs-serve` | Serves the documentation at a local address and rebuilds it when a page changes. Needs `mdbook`. |
| `make docs-reference` | Rewrites `docs/src/reference/commands.md` from the `--help` of the current code. |
| `make clean-data` | Asks for confirmation, then deletes `~/.octotask` and the folders of its projects on the other machines, unpushed work in the workspaces included. It stops before deleting `~/.octotask` when a machine cannot be reached. |
| `make install-clean` | Installs first, so a failed build leaves the data untouched, then runs `make clean-data`. |

`make clean-data` and `make install-clean` delete real data: do not run them on the folder you work with.
Try `octok` with a temporary home instead, and install it into a temporary folder:

```sh
export OCTOTASK_HOME="$(mktemp -d)"
CARGO_INSTALL_ROOT="$(mktemp -d)" make install
```

`OCTOTASK_HOME` moves everything `octok` keeps, and `CARGO_INSTALL_ROOT` moves the install, so the real `~/.octotask` and `~/.cargo/bin` stay untouched.
`make docs-reference` and the other `make` targets that run `octok` use the one built from the checkout, not the one on your `PATH`.

Run one test target with `cargo test -p octotask-core --test <adapters|config|workspaces>`, or with `cargo test -p octotask --test main` for the end-to-end tests of the binary.
Add a filter to run fewer tests: `cargo test -p octotask-core --test workspaces workspace::` runs the tests of `tests/workspaces/workspace.rs`, and a full test name runs one test.
The package of the binary is named `octotask`, not `octotask-cli`.

### What CI runs

The workflow `.github/workflows/ci.yml` runs on every pull request to `main` or `staging` and on every push to them:

- **`test`**, on Ubuntu and macOS: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, and `.github/scripts/check-version-test.sh`, the test of the release scripts.
- **`version`**, on pull requests to `main` only: fails when the version is not higher than every released one, as described in [Releases](#releases).
- **`docs`**: installs the pinned mdBook, runs `.github/scripts/build-docs-test.sh`, the test of the build script, then builds the site with `.github/scripts/build-docs.sh`.
- **`check`**: the only check that the branch rules require.
  It passes when each job above passed or was skipped.
  A new job must be added to its `needs`, or the rules will not require it.

Run the first three commands of `test` before you commit: they are what CI will run.

## The workspace

The repository is a Cargo workspace of two crates:

- `crates/octotask-core`, the library `octotask-core`: everything that is not command line.
  `src/config` holds the project files, the registry of projects and the locations; `src/adapters` calls `git`, `gh` and `ssh`; `src/workspaces` creates, completes, pulls, starts and removes workspaces and handles the dev changes; `src/error.rs` has every error, with its message, its stable code and its class.
- `crates/octotask-cli`, the package `octotask`, whose binary is `octok`: `src/main.rs` has the commands and the options, `src/prompts.rs` the questions at a terminal, `src/utils.rs` the printing and the exit codes, and `src/docs.rs` the command `octok docs`.
  `build.rs` embeds the pages of `docs/src` into the binary.

The tests mirror `src`: each crate's `tests/` has the same folders and file names as its `src/`, and a test folder is one test target, declared in the `Cargo.toml` of its crate.
The tests run the real `git` against local repositories used as remotes.
`plugins/octotask` holds the agent skill and its `plugin.json`: see [Driving octok from an agent](agents/overview.md).

## The documentation

The documentation is the set of Markdown pages in `docs/src`, listed in `docs/src/SUMMARY.md`, which is the only index: the order and the parts of the site, of `llms.txt` and of `octok docs` follow it.
The same files become four things:

- the site, built by mdBook from `docs/book.toml`, at `https://whiitex.github.io/octotask/`;
- every page as Markdown next to its HTML, for example `agents/json-output.md`;
- `llms.txt`, the index of the pages with their summary, and `llms-full.txt`, every page in one file;
- the output of `octok docs <topic>`, because `build.rs` embeds the pages in the binary.

### Conventions that tests enforce

`crates/octotask-cli/tests/docs.rs` fails the build when a page breaks one of these:

- Every page is in `SUMMARY.md` and every entry of `SUMMARY.md` has a page.
- Line 1 is `# <Title>`, exactly the title in `SUMMARY.md`, line 2 is empty and line 3 is a one-line summary: it is shown next to the title in `octok docs` and in `llms.txt`.
- No page uses an mdBook directive: a page is plain CommonMark, so the binary and the site read it in the same way.
- No page has HTML outside a code span or a code block: a placeholder such as `<branch>` written as plain text would vanish from the site, so it goes in backticks, and a link is written `[text](url)`.
- Every link to another page is a relative path to a `.md` file of the book, and the links in code blocks are not read.
- `docs/src/reference/commands.md` is the output of `octok docs reference/commands`, and covers every command, argument and option that `--help` lists, each with a help text.

Other conventions are for the writers: one full sentence per line in the source, no em dash, `##` for sections and `###` below, a language on every code fence.

### Checked examples

A code block with the language `console` is an example that a test runs against the real `octok`, so an example that no longer matches the output fails the build.
`crates/octotask-cli/tests/book` is that test.

````text
```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
```
````

- A line that starts with `$ ` is a command, and the lines below it are its expected output, stdout and stderr in the order they were printed.
- `[..]` matches any text within one line, at the start, in the middle or at the end.
  Blank lines after an output and trailing spaces are ignored.
- Each page runs in a fresh sandbox, all its blocks in order in one `sh`: `cd`, variables and files carry over from block to block.
  The text between blocks is not run.
  A page is self-contained, so its first block creates what it needs.
- The sandbox has the home `/Users/you`, `OCTOTASK_HOME` at `/Users/you/.octotask`, and the git identity `You <you@example.com>`.
  Commands get an empty stdin and no terminal: no question can be answered, `octok up` cannot run and a command still running after 120 seconds fails.
- The remotes are local repositories at the GitHub URLs `git@github.com:acme/shop-frontend.git`, `shop-backend.git` and `shop-agent.git`, with a `main` branch, a `staging` branch and a few more.
  Shop-backend has the pull request #12, read by the fake `gh`.
  Every host given to `ssh`, such as `api-dev-1`, is one fake machine whose user is `ubuntu`.
  Dates are relative to a fixed "now", so `2 days ago` never changes, and a commit made in an example is dated ten minutes ago.
- What cannot run there goes in an `sh` or `text` block.

To update the expected output of the examples after a change of behavior, let the test rewrite it, then read the diff:

```sh
BOOK_PAGE=commands/project BOOK_UPDATE=1 cargo test -p octotask --test book
```

Without `BOOK_UPDATE`, the same command only checks the page, and a failure shows the page and the line, the command, and the expected and actual output.
Leave out `BOOK_PAGE` to run every page.
The update keeps each `[..]` line that still matches.
The test binary embeds `docs/src`, so `cargo` rebuilds after each edit of a page.

### Building and publishing

`make docs` runs `.github/scripts/build-docs.sh`, which builds the HTML with `mdbook build`, then writes every page as Markdown next to it, and `llms.txt` and `llms-full.txt`, into `target/book`.
It fails before the build when `SUMMARY.md` lists a page that does not exist.
`BASE_URL` sets where the site is published, for the links of `llms.txt`.
`.github/scripts/install-mdbook.sh <folder>` installs the mdBook version that CI uses, which is pinned in that script.
`make docs-serve` serves the book while you write.

`docs/src/reference/commands.md` is generated: change the `--help` text in the code, then run `make docs-reference`.
A test fails when the committed file is stale.

The workflow `.github/workflows/docs.yml` publishes the site to GitHub Pages on every push to `main`, and when it is run by hand from the Actions tab.
Every push to `main` is a release, so the site documents the latest release.

## Releases

Every merge into `main` is released: the `Release` workflow tags the merge commit `v<version>`, builds `octok` for the four platforms and publishes a GitHub release with the archives and a file of checksums.
The tag is created only after every build succeeded, so a failed workflow leaves no tag: fix it with a hotfix that keeps the same version.
Never create or move a `v*` tag by hand.

The version is the `version` under `[workspace.package]` in the root `Cargo.toml`, a plain `X.Y.Z` shared by both crates.
A pull request to `main` fails its `version` check unless it is higher than every released one.
Raise it before opening the pull request: edit the root `Cargo.toml`, run `cargo check` so that `Cargo.lock` follows, and set the same version in `plugins/octotask/plugin.json`, because a test fails when the two differ.
While the version is `0.y.z`, raise `y` for new or changed behavior and `z` for fixes only.

### Branches

Work happens on a branch from an up-to-date `staging`, named `feat/<topic>` or `fix/<topic>`, and goes into `staging` with a pull request merged by squash, so each feature is one commit there.
Titles follow Conventional Commits, such as `feat: add the repo cache`.
`staging` goes into `main` with a pull request titled `release: <what it contains>`, merged with a merge commit, never squashed or rebased: `main` would get commits that `staging` does not have, and the next promotion would conflict.
A fix that cannot wait branches from `main`, goes to `main` by squash with a raised patch version, and comes back to `staging` with a merge commit.
