#!/usr/bin/env bash
# Fails unless <version> can be released from the repository in the current folder:
# it must be a plain X.Y.Z, higher than every release tag `vX.Y.Z` the repository has.
set -euo pipefail

RELEASE='(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)'

main() {
    local version=${1-}
    if ! [[ $version =~ ^$RELEASE$ ]]; then
        reject "version \`$version\` is not in the form X.Y.Z"
    fi

    local tag
    for tag in $(git tag --list 'v*'); do
        if [[ $tag =~ ^v$RELEASE$ ]] && ! is_higher "$version" "${tag#v}"; then
            reject "version $version is not higher than the release $tag: raise \`version\` in Cargo.toml"
        fi
    done
}

# True when version $1 is higher than version $2.
is_higher() {
    local IFS=.
    local -a left=($1) right=($2)
    local i
    for i in 0 1 2; do
        if ((left[i] > right[i])); then return 0; fi
        if ((left[i] < right[i])); then return 1; fi
    done
    return 1
}

reject() {
    echo "error: $1" >&2
    exit 1
}

main "$@"
