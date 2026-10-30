#!/bin/sh
# Pinned development tools; archive SHA-256s checked against GitHub release digests.
set -eu
if [ "$#" != 1 ] || [ -z "$1" ]; then
    echo "usage: install-analysis-tools.sh DESTINATION" >&2
    exit 1
fi
case "$(uname -s)-$(uname -m)" in
    Darwin-arm64)
        action_platform=darwin_arm64
        action_sha=aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f
        shell_platform=darwin.aarch64
        shell_sha=339b930feb1ea764467013cc1f72d09cd6b869ebf1013296ba9055ab2ffbd26f
        ;;
    Linux-x86_64)
        action_platform=linux_amd64
        action_sha=8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8
        shell_platform=linux.x86_64
        shell_sha=b7af85e41cc99489dcc21d66c6d5f3685138f06d34651e6d34b42ec6d54fe6f6
        ;;
    *) echo "unsupported analysis-tool platform" >&2; exit 1 ;;
esac
mkdir -p "$1/bin"
destination=$(cd "$1" && pwd)
scratch=$(mktemp -d "$destination/download.XXXXXX")
cleanup() {
    rm -f "$scratch/actionlint.tar.gz" "$scratch/shellcheck.tar.gz"
    rmdir "$scratch"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
curl --proto '=https' --tlsv1.2 -fsSL \
    "https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_${action_platform}.tar.gz" \
    -o "$scratch/actionlint.tar.gz"
curl --proto '=https' --tlsv1.2 -fsSL \
    "https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.${shell_platform}.tar.gz" \
    -o "$scratch/shellcheck.tar.gz"
printf '%s  %s\n' "$action_sha" "$scratch/actionlint.tar.gz" "$shell_sha" "$scratch/shellcheck.tar.gz" |
    shasum -a 256 -c - >&2
tar -xOf "$scratch/actionlint.tar.gz" actionlint > "$destination/bin/actionlint"
tar -xOf "$scratch/shellcheck.tar.gz" shellcheck-v0.11.0/shellcheck > "$destination/bin/shellcheck"
chmod +x "$destination/bin/actionlint" "$destination/bin/shellcheck"
printf '%s\n' "$destination/bin"
