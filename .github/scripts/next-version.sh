#!/bin/sh
# Print the next release tag: v{YY}.{M}.{build}, where the build number counts releases
# in that month and year and starts at 0. Uses UTC, like the workflows.
set -eu

if [ "$#" -gt 1 ]; then
    echo "usage: next-version.sh [YYYY-MM]" >&2
    exit 1
fi
period="${1:-$(date -u +%Y-%m)}"
case "$period" in
    [0-9][0-9][0-9][0-9]-0[1-9]|[0-9][0-9][0-9][0-9]-1[0-2]) ;;
    *) echo "invalid UTC period: $period (expected YYYY-MM)" >&2; exit 1 ;;
esac
year=$(printf '%s' "$period" | cut -c3-4)
month="${period#????-}"
month="${month#0}"
prefix="v${year}.${month}."

latest=$(
    git tag --list "${prefix}*" |
        sed 's/^[^.]*\.[^.]*\.//' |
        grep -E '^(0|[1-9][0-9]*)$' |
        sort -n |
        tail -n 1 || true
)

if [ -z "${latest}" ]; then
    echo "${prefix}0"
else
    echo "${prefix}$((latest + 1))"
fi
