# Installation

What `octok` needs on your machine, how to install it from a release or from source, and how to update, remove and relocate it.

## Requirements

| Requirement | Needed for |
|---|---|
| macOS or Linux | Everything. There is no Windows build. |
| `git` 2.37 or newer | Everything, also on the other machine of a [location](../concepts.md#location). |
| `ssh` | Only for repositories on another machine: see [Locations and SSH](../commands/locations.md). |
| `gh`, signed in with `gh auth login` | Only for `octok pr review`. |
| [Rust](https://rustup.rs) 1.89 or newer | Only to build from source. |

## From a release

Every [release](https://github.com/whiitex/octotask/releases) has one archive per platform, holding a folder of the same name with the `octok` binary, the `LICENSE` and a `README.md`:

| Platform | Archive |
|---|---|
| macOS, Apple silicon | `octok-aarch64-apple-darwin.tar.gz` |
| macOS, Intel | `octok-x86_64-apple-darwin.tar.gz` |
| Linux, arm64 | `octok-aarch64-unknown-linux-musl.tar.gz` |
| Linux, x86_64 | `octok-x86_64-unknown-linux-musl.tar.gz` |

The Linux binaries are statically linked, so they run on any distribution.
Download the archive of your platform, unpack it and move `octok` into a folder of your `PATH`, using your archive name in both places:

```sh
curl -fsSL https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz | tar -xz
sudo mkdir -p /usr/local/bin
sudo mv octok-aarch64-apple-darwin/octok /usr/local/bin/
```

For a specific release, replace `latest/download` with `download/v<version>`, for example `download/v0.1.0`.

### Checksums

The file `SHA256SUMS` of the release has the checksum of every archive.
Check the archive before you unpack it:

```sh
curl -fsSLO https://github.com/whiitex/octotask/releases/latest/download/octok-aarch64-apple-darwin.tar.gz
curl -fsSLO https://github.com/whiitex/octotask/releases/latest/download/SHA256SUMS
grep ' octok-aarch64-apple-darwin.tar.gz$' SHA256SUMS | shasum -a 256 -c
```

It prints `octok-aarch64-apple-darwin.tar.gz: OK` for a good archive.
On Linux use `sha256sum -c` in place of `shasum -a 256 -c`.

## From source

Clone the repository and install `octok` into `~/.cargo/bin` with `make install`, which runs `cargo install --path crates/octotask-cli --locked`:

```sh
git clone https://github.com/whiitex/octotask.git
cd octotask
make install
```

`~/.cargo/bin` must be in your `PATH`: `rustup` adds it, and `. "$HOME/.cargo/env"` does it for the current shell.
To install somewhere else, set `CARGO_INSTALL_ROOT` to a folder: the binary goes in its `bin` folder.

## After installing

Check the version:

```console
$ octok --version
octok [..]
```

There is nothing to set up: `octok` creates its data folder the first time a command needs it.
Then follow the [Quick start](quick-start.md), or run `octok help` for the commands.
`octok docs` prints these pages for the installed version: see [Built-in documentation](../commands/docs.md).
For a coding agent, install the skill described in [Driving octok from an agent](../agents/overview.md).

## Updating

Replace the binary, from a new release archive or with `git pull` and `make install` in the clone.
Your projects, clones and workspaces stay as they are.
A binary that does not know the [`version`](../reference/configuration.md#the-version-field) of a config file refuses it with `unsupported config version` instead of misreading it.

## Removing

Remove the projects first, so their folders on other machines are deleted too, then the data folder and the binary:

```sh
octok project ls
octok project rm shop
rm -r ~/.octotask
sudo rm /usr/local/bin/octok
```

- `octok project rm` refuses while the project has workspaces, because their unpushed work would be lost: `--force` deletes them anyway.
- Deleting `~/.octotask` by hand also deletes its workspaces, with any work never pushed.
- For a binary installed with `make install`, run `cargo uninstall octotask` in place of the last command.
- The control sockets in `~/.ssh/octok-*` go away by themselves a minute after the last command.

## The data folder

`octok` keeps everything it manages under `~/.octotask`: the projects, the clones, the workspaces and the dev changes.
Set `OCTOTASK_HOME` to use another folder:

```sh
export OCTOTASK_HOME=/data/octotask
```

An empty `OCTOTASK_HOME` counts as unset.
Another folder is a separate world, with its own projects, default project and workspaces, which makes it handy for trying `octok` safely.
On another machine the folder is the one of the [location](../concepts.md#location), `.octotask` in the home of the user there by default.
[Configuration files](../reference/configuration.md) describes every file in it.
