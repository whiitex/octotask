# Installation

What `octok` needs on your machine, how to install it from a release or from source, and how to update, remove and relocate it.

## Requirements

| Requirement | Needed for |
|---|---|
| macOS or Linux | Everything. There is no Windows build. |
| `git` 2.37 or newer | Everything: `octok` runs your own `git` for the clones, the worktrees and every branch operation. |
| `ssh` | Only to keep repositories on another machine: see [Locations and SSH](../commands/locations.md). |
| `gh`, the GitHub CLI, signed in with `gh auth login` | Only for `octok pr review`, which reads the branch of a pull request with it. |
| [Rust](https://rustup.rs) 1.89 or newer | Only to build from source. |

The other machine of a [location](../concepts.md#location) needs `git` 2.37 or newer too.
`octok` uses your own `git`, so your credentials, SSH configuration and hooks apply to every command.

## From a release

Every [release](https://github.com/whiitex/octotask/releases) has one archive per platform:

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `octok-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `octok-x86_64-apple-darwin.tar.gz` |
| Linux, arm64 | `octok-aarch64-unknown-linux-musl.tar.gz` |
| Linux, x86_64 | `octok-x86_64-unknown-linux-musl.tar.gz` |

An archive holds a folder with the same name, with the `octok` binary, the `LICENSE` and a `README.md`.
Download the archive of the latest release, unpack it and move `octok` into a folder of your `PATH`:

```sh
curl -fsSL https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz | tar -xz
sudo mkdir -p /usr/local/bin
sudo mv octok-aarch64-apple-darwin/octok /usr/local/bin/
```

Use the archive of your platform in both places.
To install a specific release, replace `latest/download` with `download/v<version>`, for example `download/v0.1.0`.

The Linux binaries are statically linked, so they run on any distribution.

### Checksums

The release also has a file `SHA256SUMS` with the SHA-256 checksum of every archive.
Download it next to the archive and check it before you unpack:

```sh
curl -fsSLO https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz
curl -fsSLO https://github.com/whiitex/octotask/releases/latest/download/SHA256SUMS
grep ' octok-aarch64-apple-darwin.tar.gz$' SHA256SUMS | shasum -a 256 -c
```

On Linux use `sha256sum -c` in place of `shasum -a 256 -c`.
The command prints `octok-aarch64-apple-darwin.tar.gz: OK` when the archive is the one the release published.

## From source

Building needs Rust 1.89 or newer, which is the `rust-version` of the crates.
Clone the repository and install the `octok` binary into `~/.cargo/bin`:

```sh
git clone https://github.com/whiitex/octotask.git
cd octotask
make install
```

`make install` runs `cargo install --path crates/octotask-cli --locked`.
Make sure `~/.cargo/bin` is in your `PATH`: `rustup` adds it, and `. "$HOME/.cargo/env"` does it for the current shell.
To install somewhere else, set `CARGO_INSTALL_ROOT` to a folder: the binary goes in its `bin` folder.

## After installing

Check that the binary runs and which version you have:

```console
$ octok --version
octok [..]
```

Then follow the [Quick start](quick-start.md), or run `octok help` for the list of commands.
The documentation of the installed version is in the binary: `octok docs` lists its pages and `octok docs <topic>` prints one, as described in [Built-in documentation](../commands/docs.md).

`octok` creates its data folder the first time a command needs it: there is nothing to set up.

If you use a coding agent, install the skill that teaches it to drive `octok`: see [Driving octok from an agent](../agents/overview.md).

## Updating

Replacing the binary does not touch the data folder, so your projects, clones and workspaces stay as they are.
The project files and `config.yaml` carry a `version` field (see [Configuration files](../reference/configuration.md#the-version-field)), and a binary that does not know that version refuses the file with `unsupported config version` instead of misreading it.

- **From a release:** download the new archive and replace the binary, with the commands above.
- **From source:** update the clone and install again:

```sh
cd octotask
git pull
make install
```

Check the result with `octok --version`.

## Removing

Remove the projects first, so that the folders they have on other machines are deleted too.
`octok project rm <name>` deletes a project and the clones of its repositories on every machine.
It refuses while the project has workspaces, because their unpushed work would be lost: `--force` deletes them anyway.
Then remove the binary and what is left of the data folder:

```sh
octok project ls
octok project rm shop
rm -r ~/.octotask
sudo rm /usr/local/bin/octok
```

For a binary installed with `make install`, run `cargo uninstall octotask` in place of the last command: the package is called `octotask` and the binary `octok`.
Deleting `~/.octotask` by hand also deletes the workspaces in it, with any work that was never pushed.
The control sockets `octok` leaves in `~/.ssh/octok-*` are removed by themselves a minute after the last command.

## The data folder

`octok` keeps everything it manages under `~/.octotask`: the list of projects, the clone of each repository, the workspaces and the dev changes.
Set the `OCTOTASK_HOME` environment variable to use another folder:

```sh
export OCTOTASK_HOME=/data/octotask
```

An empty `OCTOTASK_HOME` counts as unset.
A different folder is a separate world: its projects, its default project and its workspaces are independent of `~/.octotask`, which makes it handy for trying `octok` without touching your real projects.

| Path | Content |
|---|---|
| `~/.octotask/config.yaml` | The default project. |
| `~/.octotask/projects/<project>.yaml` | The repositories of a project. |
| `~/.octotask/cache/<project>/<repo>.git` | The bare clone of a repository. |
| `~/.octotask/workspaces/<project>/<workspace>/<repo>` | The checkouts of a workspace. |
| `~/.octotask/dev/<project>/<repo>` | The dev changes saved for a repository. |

The folder of another machine is the one in the [location](../concepts.md#location) of its repositories, `.octotask` in the home of the user there by default.
[Configuration files](../reference/configuration.md) describes every file in it.
