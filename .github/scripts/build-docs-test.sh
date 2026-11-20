#!/bin/sh
# Tests of build-docs.sh: each case builds a small fixture book in a throwaway folder.
set -u

script="$(cd "$(dirname "$0")" && pwd)/build-docs.sh"
base=https://docs.example.com/site/
failures=0
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

main() {
    make_book "$work/book"
    if ! BASE_URL=$base "$script" "$work/book" "$work/out" >"$work/build.log" 2>&1; then
        cat "$work/build.log"
        fail "the fixture book builds"
        finish
    fi
    echo "ok: the fixture book builds"

    llms_has_title_and_description
    llms_groups_the_pages_by_part_in_order
    llms_links_are_absolute_md_urls
    llms_pages_without_summary_have_no_colon
    md_is_served_next_to_html
    llms_full_has_every_page_in_order
    default_base_url_is_the_published_site
    missing_page_fails_the_build

    finish
}

llms_has_title_and_description() {
    expect_equal "llms.txt starts with the title and the description" \
        "$(head -n 3 "$work/out/llms.txt")" \
        "# Fixture

> A book to test the build"
}

llms_groups_the_pages_by_part_in_order() {
    expect_equal "llms.txt groups the pages by part, in SUMMARY.md order" \
        "$(grep -E '^## |^- ' "$work/out/llms.txt" | sed 's/(.*//')" \
        "## Overview
- [Introduction]
## Getting started
- [Install]
- [Quick start]
## Commands
- [Project]"
}

llms_links_are_absolute_md_urls() {
    expect_contains "llms.txt links the absolute URL of the .md page, with its summary" \
        "$(cat "$work/out/llms.txt")" \
        "- [Install](${base}getting-started/install.md): How to install the fixture."
}

llms_pages_without_summary_have_no_colon() {
    expect_contains "a page without a summary is listed without a colon" \
        "$(cat "$work/out/llms.txt")" \
        "- [Quick start](${base}getting-started/quick-start.md)
"
}

md_is_served_next_to_html() {
    for page in introduction getting-started/install getting-started/quick-start commands/project; do
        if [ -f "$work/out/$page.html" ] && cmp -s "$work/out/$page.md" "$work/book/src/$page.md"; then
            echo "ok: $page.md is served next to $page.html"
        else
            fail "$page.md is served next to $page.html"
        fi
    done
}

llms_full_has_every_page_in_order() {
    expect_equal "llms-full.txt has every page, in order" \
        "$(grep '^# ' "$work/out/llms-full.txt")" \
        "# Introduction
# Install
# Quick start
# Project"
    expect_contains "llms-full.txt has the text of the pages" \
        "$(cat "$work/out/llms-full.txt")" \
        "How to install the fixture."
}

default_base_url_is_the_published_site() {
    (unset BASE_URL; "$script" "$work/book" "$work/default" >/dev/null 2>&1)
    expect_contains "the base URL defaults to the published site" \
        "$(cat "$work/default/llms.txt")" \
        "(https://whiitex.github.io/octotask/introduction.md)"
}

missing_page_fails_the_build() {
    make_book "$work/broken"
    echo "- [Ghost](commands/ghost.md)" >> "$work/broken/src/SUMMARY.md"
    if "$script" "$work/broken" "$work/broken-out" >"$work/broken.log" 2>&1; then
        fail "a page listed in SUMMARY.md but missing fails the build"
    elif grep -q 'commands/ghost.md' "$work/broken.log" && [ ! -e "$work/broken/src/commands/ghost.md" ]; then
        echo "ok: a page listed in SUMMARY.md but missing fails the build, and is not created"
    else
        fail "a missing page is named in the error and not created: $(cat "$work/broken.log")"
    fi
}

make_book() {
    mkdir -p "$1/src/getting-started" "$1/src/commands"
    cat > "$1/book.toml" <<'TOML'
[book]
title = "Fixture"
description = "A book to test the build"
src = "src"

[build]
create-missing = false
TOML
    cat > "$1/src/SUMMARY.md" <<'MD'
# Summary

[Introduction](introduction.md)

# Getting started

- [Install](getting-started/install.md)
- [Quick start](getting-started/quick-start.md)

# Commands

- [Project](commands/project.md)
MD
    page "$1/src/introduction.md" Introduction "What the fixture is."
    page "$1/src/getting-started/install.md" Install "How to install the fixture."
    page "$1/src/getting-started/quick-start.md" "Quick start" ""
    page "$1/src/commands/project.md" Project "The project command."
}

# Writes a page: the title, an empty line and the summary, then some text.
page() {
    printf '# %s\n\n%s\n\nBody of %s.\n' "$2" "$3" "$2" > "$1"
}

expect_equal() {
    if [ "$2" = "$3" ]; then
        echo "ok: $1"
    else
        fail "$1, got:
$2"
    fi
}

expect_contains() {
    case $2 in
        *"$3"*) echo "ok: $1" ;;
        *) fail "$1, got:
$2" ;;
    esac
}

fail() {
    echo "FAILED: $1"
    failures=$((failures + 1))
}

finish() {
    if [ "$failures" -gt 0 ]; then
        echo "$failures failed"
        exit 1
    fi
    echo "all passed"
    exit 0
}

main
