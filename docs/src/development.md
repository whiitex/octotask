# Development

How to build, test and release `octok`, how the workspace is laid out, and how these pages are written, checked and published.

## Building and testing

You need [Rust](https://rustup.rs) 1.89 or newer and `git`, and the tests need `gh` too.
The tests use no network: the remotes are local repositories, and a fake `ssh` and a fake `gh` stand in for the other machine and GitHub.
If `cargo` is not on `PATH`, run `. "$HOME/.cargo/env"` first.

| Command | What it does |
|---|---|
| `make format` | Formats the code. |
| `make test` | Runs every test. |
| `make build` | Builds the release binary. |
| `make install` | Runs `cargo install --path crates/octotask-cli --locked`, into `~/.cargo/bin`. |
| `make docs` | Builds the site into `target/book`. Needs `mdbook` on `PATH`. |
| `make docs-serve` | Serves the site locally and rebuilds it on every change. Needs `mdbook`. |
| `make docs-reference` | Rewrites `docs/src/reference/commands.md` from the `--help` of the code. |
| `make clean-data` | After a confirmation, deletes `~/.octotask` and its project folders on other machines, unpushed work included. It keeps `~/.octotask` when a machine cannot be reached. |
| `make install-clean` | Runs `make install`, then `make clean-data` only if the build succeeded. |

The targets that run `octok` use the binary built from the checkout, not the one on your `PATH`.
Never run `make clean-data` or `make install-clean` on the data you work with.
To try `octok`, use a temporary home and install folder, so `~/.octotask` and `~/.cargo/bin` stay untouched:

```sh
export OCTOTASK_HOME="$(mktemp -d)"
CARGO_INSTALL_ROOT="$(mktemp -d)" make install
```

Run one test target with `cargo test -p octotask-core --test <adapters|config|workspaces>`, or the end-to-end tests of the binary with `cargo test -p octotask --test main`.
The package of the binary is `octotask`, not `octotask-cli`.
Add a filter to run fewer tests: `cargo test -p octotask-core --test workspaces workspace::` runs `tests/workspaces/workspace.rs`, and a full test name runs one test.

### What CI runs

`.github/workflows/ci.yml` runs on every pull request and push to `main` or `staging`:

| Job | What it checks |
|---|---|
| `test`, on Ubuntu and macOS | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` and `.github/scripts/check-version-test.sh`. |
| `version`, on pull requests to `main` | The version is higher than every released one: see [Releases](#releases). |
| `docs` | Runs `.github/scripts/build-docs-test.sh`, then builds the site with `.github/scripts/build-docs.sh` and the pinned mdBook. |
| `check` | Passes when every job above passed or was skipped. It is the only check the branch rules require, so add every new job to its `needs`. |

Run the first three commands of `test` before you commit.

## The workspace

- `crates/octotask-core` is the library: `src/config` for the project files, the registry and the locations, `src/adapters` for `git`, `gh` and `ssh`, `src/workspaces` for the workspaces and the dev changes, and `src/error.rs` for every error with its message, stable code and class.
- `crates/octotask-cli` is the package `octotask` with the binary `octok`: `src/main.rs` for the commands, `src/prompts.rs` for the questions, `src/utils.rs` for printing and exit codes, `src/docs.rs` for `octok docs`, and `build.rs` to embed `docs/src` in the binary.
- `plugins/octotask` holds the agent skill and its `plugin.json`: see [Driving octok from an agent](agents/overview.md).

Each crate's `tests/` mirrors its `src/`, and each test folder is one test target declared in the crate's `Cargo.toml`.

## The documentation

The pages are the Markdown files of `docs/src`, in the order of `docs/src/SUMMARY.md`, the only index.
They become the site at `https://whiitex.github.io/octotask/`, each page as Markdown next to its HTML, `llms.txt` (the index with the summaries), `llms-full.txt` (every page in one file) and the output of `octok docs <topic>`.

`crates/octotask-cli/tests/docs.rs` checks that:

- every page is in `SUMMARY.md`, and every entry of it has a page;
- line 1 is `# <Title>` as in `SUMMARY.md`, line 2 is empty and line 3 is a one-line summary;
- no page uses an mdBook directive or HTML outside code, so a placeholder such as `<branch>` goes in backticks;
- every link to another page is a relative path to a `.md` file of the book;
- `docs/src/reference/commands.md` is what `make docs-reference` writes, and every command, argument and option has a help text.

Write one full sentence per line, no em dash, `##` and `###` headings, and a language on every code fence.

### Checked examples

`crates/octotask-cli/tests/book` runs every `console` block against the real `octok`:

````text
```console
$ octok project init shop
created project `shop` (base: staging) at /Users/you/.octotask/projects/shop.yaml
`shop` is now the default project
```
````

- **Format:** a line starting with `$ ` is a command, and the lines below it are its stdout and stderr in order.
  `[..]` matches any text within one line, and blank lines after an output and trailing spaces are ignored.
- **State:** each page runs in a fresh sandbox, all its blocks in one `sh`, so `cd`, variables and files carry over; the first block creates what the page needs.
- **Sandbox:** the home is `/Users/you`, `OCTOTASK_HOME` is `/Users/you/.octotask` and the git identity is `You <you@example.com>`.
  Commands get an empty stdin and no terminal, so no question can be answered, `octok up` cannot run and a command still running after 120 seconds fails.
- **Fixtures:** the remotes `git@github.com:acme/shop-frontend.git`, `shop-backend.git` and `shop-agent.git` are local, with `main`, `staging` and a few more branches, and `shop-backend` has the pull request #12.
  Every `ssh` host, such as `api-dev-1`, is one fake machine with the user `ubuntu`.
  Dates are relative to a fixed "now", and a commit made in an example is ten minutes old.
- **Not runnable:** use an `sh` or `text` block.

After a change of behavior, let the test rewrite the expected output, then read the diff:

```sh
BOOK_PAGE=commands/project BOOK_UPDATE=1 cargo test -p octotask --test book
```

Without `BOOK_UPDATE` it only checks, and a failure shows the page, the line, the command and both outputs.
Leave out `BOOK_PAGE` to run every page.
The update keeps each `[..]` line that still matches.

### Building and publishing

`make docs` runs `.github/scripts/build-docs.sh`, which runs `mdbook build` and adds the Markdown pages, `llms.txt` and `llms-full.txt` to `target/book`.
It fails when `SUMMARY.md` lists a missing page, and `BASE_URL` sets the site address used in `llms.txt`.
`.github/scripts/install-mdbook.sh <folder>` installs the mdBook version that CI pins.
`.github/workflows/docs.yml` publishes the site to GitHub Pages on every push to `main`, so it documents the latest release, and when run by hand.

## Releases

Every merge into `main` is a release: the `Release` workflow builds `octok` for the four platforms, then tags the merge commit `v<version>` and publishes the archives and their checksums.
A failed workflow leaves no tag, so fix it with a hotfix that keeps the same version.
Never create or move a `v*` tag by hand.

The version is `version` under `[workspace.package]` in the root `Cargo.toml`, a plain `X.Y.Z`.
Raise it before opening a pull request to `main`, or its `version` check fails:

1. Edit the root `Cargo.toml`.
2. Run `cargo check`, so `Cargo.lock` follows.
3. Set the same version in `plugins/octotask/plugin.json`, or a test fails.

While the version is `0.y.z`, raise `y` for new or changed behavior and `z` for fixes only.

### Branches

- **Features:** branch `feat/<topic>` or `fix/<topic>` from an up-to-date `staging`, and merge it into `staging` by squash, with a Conventional Commits title such as `feat: add the repo cache`.
- **Releases:** merge `staging` into `main` with a merge commit, titled `release: <what it contains>`.
  A squash or a rebase would give `main` commits that `staging` lacks, and the next promotion would conflict.
- **Hotfixes:** branch from `main`, merge into `main` by squash with a raised patch version, then merge `main` into `staging` with a merge commit.
