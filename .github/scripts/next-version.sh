#!/bin/sh
# Print the next release tag: v{YY}.{M}.{build}, where the build number counts releases
# in that month and year and starts at 0. Uses UTC, like the workflows.
set -eu

year=$(date -u +%y)
month=$(date -u +%m | sed -e 's/^0//')
prefix="v${year}.${month}."

latest=$(
    git tag --list "${prefix}*" |
        sed -e "s/^${prefix}//" |
        grep -E '^[0-9]+$' |
        sort -n |
        tail -n 1 || true
)

if [ -z "${latest}" ]; then
    echo "${prefix}0"
else
    echo "${prefix}$((latest + 1))"
fi
