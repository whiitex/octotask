#!/usr/bin/env bash
# Tests of check-version.sh: each case runs it in a throwaway repository with the given tags.
set -uo pipefail

script="$(cd "$(dirname "$0")" && pwd)/check-version.sh"
failures=0

main() {
    accepts "the first release" 0.1.0
    accepts "a higher patch" 0.1.1 v0.1.0
    accepts "a higher minor" 0.2.0 v0.1.9
    accepts "a higher major" 1.0.0 v0.9.9
    accepts "numbers compared as numbers" 0.10.0 v0.9.0
    accepts "tags that are not releases" 0.1.0 vendor v0.2.0-rc1 0.5.0

    rejects "a released version" 0.1.0 v0.1.0
    rejects "a version below the latest release" 0.1.5 v0.1.0 v0.2.0
    rejects "a lower patch" 0.1.0 v0.1.1
    rejects "a lower major with a higher minor" 1.9.0 v2.0.0
    rejects "a version without patch" 1.0
    rejects "a pre-release" 1.0.0-rc1
    rejects "a version with the tag prefix" v1.0.0
    rejects "a leading zero" 01.0.0
    rejects "an empty version" ""

    rejection_names_the_release

    if [ "$failures" -gt 0 ]; then
        echo "$failures failed"
        exit 1
    fi
    echo "all passed"
}

accepts() {
    local name=$1 version=$2
    shift 2
    if run_check "$version" "$@" >/dev/null 2>&1; then
        echo "ok: accepts $name"
    else
        fail "accepts $name"
    fi
}

rejects() {
    local name=$1 version=$2
    shift 2
    if run_check "$version" "$@" >/dev/null 2>&1; then
        fail "rejects $name"
    else
        echo "ok: rejects $name"
    fi
}

rejection_names_the_release() {
    local output
    output=$(run_check 0.1.5 v0.1.0 v0.2.0 2>&1)
    case "$output" in
        *v0.2.0*) echo "ok: rejection names the release" ;;
        *) fail "rejection names the release, got: $output" ;;
    esac
}

fail() {
    echo "FAILED: $1"
    failures=$((failures + 1))
}

# Runs the script on <version> in a new repository that has the tags that follow.
run_check() {
    local version=$1 repo status
    shift
    repo=$(mktemp -d)
    git -C "$repo" init --quiet
    git -C "$repo" -c user.name=test -c user.email=test@example.com -c commit.gpgsign=false \
        commit --quiet --allow-empty --message init
    for tag in "$@"; do
        git -C "$repo" -c tag.gpgsign=false tag "$tag"
    done
    (cd "$repo" && "$script" "$version")
    status=$?
    rm -rf "$repo"
    return $status
}

main
