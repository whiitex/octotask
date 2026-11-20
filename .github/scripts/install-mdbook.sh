#!/bin/sh
# Installs the pinned mdBook release into <folder> and, on GitHub Actions, adds it to PATH.
# This is the one place that says which mdBook version the documentation is built with.
set -eu

MDBOOK_VERSION=0.5.4

main() {
    folder=${1:?usage: install-mdbook.sh <folder>}
    target=$(release_target)
    url="https://github.com/rust-lang/mdBook/releases/download/v$MDBOOK_VERSION/mdbook-v$MDBOOK_VERSION-$target.tar.gz"

    mkdir -p "$folder"
    curl --fail --silent --show-error --location "$url" | tar -xzf - -C "$folder"
    "$folder/mdbook" --version

    if [ -n "${GITHUB_PATH-}" ]; then
        echo "$folder" >> "$GITHUB_PATH"
    fi
}

# The Linux builds are static (musl), so they run on any runner.
release_target() {
    case "$(uname -s)-$(uname -m)" in
        Darwin-arm64) echo aarch64-apple-darwin ;;
        Darwin-x86_64) echo x86_64-apple-darwin ;;
        Linux-aarch64 | Linux-arm64) echo aarch64-unknown-linux-musl ;;
        Linux-x86_64) echo x86_64-unknown-linux-musl ;;
        *)
            echo "error: no mdBook release for $(uname -s) $(uname -m)" >&2
            exit 1
            ;;
    esac
}

main "$@"
