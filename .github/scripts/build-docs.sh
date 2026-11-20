#!/bin/sh
# Builds the documentation site from <book-dir> into <out-dir>:
# the mdBook HTML, every page as Markdown next to its HTML, llms.txt and llms-full.txt.
# SUMMARY.md is the only index: the order and the parts of the site follow it.
# BASE_URL is where the site is published, and is used for the links of llms.txt.
set -eu

BASE_URL=${BASE_URL:-https://whiitex.github.io/octotask/}

main() {
    book_dir=${1:-docs}
    out_dir=${2:-target/book}
    case $BASE_URL in
        */) ;;
        *) BASE_URL=$BASE_URL/ ;;
    esac
    src_dir=$book_dir/src

    check_pages_exist "$src_dir"
    mdbook build "$book_dir" --dest-dir "$out_dir"
    write_markdown_files "$src_dir" "$out_dir"
}

# Fails before the build when SUMMARY.md lists a page that does not exist.
check_pages_exist() {
    each_page "$1/SUMMARY.md" | while IFS= read -r path; do
        if [ ! -f "$1/$path" ]; then
            echo "error: SUMMARY.md lists $path, which does not exist in $1" >&2
            exit 1
        fi
    done
}

# Prints the path of every page of a SUMMARY.md, in order.
each_page() {
    sed -n 's/^[[:space:]]*-\{0,1\}[[:space:]]*\[.*\](\([^)]*\))[[:space:]]*$/\1/p' "$1"
}

write_markdown_files() {
    src_dir=$1
    out_dir=$2

    write_llms_header "$src_dir/../book.toml" > "$out_dir/llms.txt"
    : > "$out_dir/llms-full.txt"

    section=
    while IFS= read -r line; do
        case $line in
            "# Summary" | "") ;;
            "# "*)
                section=${line#"# "}
                printf '\n## %s\n\n' "$section" >> "$out_dir/llms.txt"
                ;;
            *)
                entry=$(printf '%s\n' "$line" | sed -n 's/^[[:space:]]*-\{0,1\}[[:space:]]*\[\(.*\)\](\([^)]*\))[[:space:]]*$/\1|\2/p')
                [ -n "$entry" ] || continue
                # The prefix chapters come before the first part and have no heading of their own.
                if [ -z "$section" ]; then
                    section=Overview
                    printf '\n## Overview\n\n' >> "$out_dir/llms.txt"
                fi
                add_page "$src_dir" "$out_dir" "${entry%|*}" "${entry##*|}"
                ;;
        esac
    done < "$src_dir/SUMMARY.md"
}

add_page() {
    src_dir=$1 out_dir=$2 title=$3 path=$4
    mkdir -p "$out_dir/$(dirname "$path")"
    cp "$src_dir/$path" "$out_dir/$path"

    summary=$(sed -n '3s/[[:space:]]*$//p' "$src_dir/$path")
    if [ -n "$summary" ]; then
        printf -- '- [%s](%s%s): %s\n' "$title" "$BASE_URL" "$path" "$summary" >> "$out_dir/llms.txt"
    else
        printf -- '- [%s](%s%s)\n' "$title" "$BASE_URL" "$path" >> "$out_dir/llms.txt"
    fi
    cat "$src_dir/$path" >> "$out_dir/llms-full.txt"
    printf '\n' >> "$out_dir/llms-full.txt"
}

# The H1 and the blockquote of llms.txt: the title and the description of book.toml.
write_llms_header() {
    title=$(toml_value title "$1")
    description=$(toml_value description "$1")
    printf '# %s\n\n> %s\n' "$title" "$description"
}

# Prints the string value of a top-level key written as `key = "value"`.
toml_value() {
    sed -n "s/^$1 = \"\(.*\)\"[[:space:]]*\$/\1/p" "$2" | head -n 1
}

main "$@"
