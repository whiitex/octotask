#!/usr/bin/env bash
# Prints the version of the `octotask` package, the one `octok --version` reports.
set -euo pipefail

cargo metadata --no-deps --format-version 1 --locked |
    jq --exit-status --raw-output '.packages[] | select(.name == "octotask") | .version'
