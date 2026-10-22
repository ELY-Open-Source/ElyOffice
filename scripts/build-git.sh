#!/usr/bin/env bash
# Builds the pinned Git, relocatable, into engines/git.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
src=$root/engines/git-src
out=$root/engines/git
jobs=$(sysctl -n hw.ncpu 2>/dev/null || nproc)

# Windows: Git for Windows' MinGit at the same version, checked.
case $(uname -s) in
  MINGW* | MSYS*)
    asset=MinGit-2.56.0-64-bit.zip
    sum=064b440ff870ed5198527e8f3a92cdf5bd2fd0fedf5e718af95e3fdaddeff718
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    curl -fsSL -o "$work/$asset" "https://github.com/git-for-windows/git/releases/download/v2.56.0.windows.1/$asset"
    got=$(shasum -a 256 "$work/$asset" | cut -d' ' -f1)
    [ "$got" = "$sum" ] || {
      echo "build-git.sh: $asset hash $got, want $sum" >&2
      exit 1
    }
    rm -rf "$out"
    unzip -q "$work/$asset" -d "$out"
    "$out/cmd/git.exe" --version
    exit 0
    ;;
esac

# reference/ stays read-only; the build runs in a copy.
mkdir -p "$src"
rsync -a --delete --exclude .git "$root/reference/git/" "$src/"
# Relocatable; built-ins are symlinks, so bundles copy one binary.
flags="prefix=$out RUNTIME_PREFIX=YesPlease INSTALL_SYMLINKS=YesPlease NO_GETTEXT=YesPlease NO_TCLTK=YesPlease NO_PERL=YesPlease NO_PYTHON=YesPlease"
# The bundles say macOS 14; Git must not need newer.
[ "$(uname)" = Darwin ] && export MACOSX_DEPLOYMENT_TARGET=14.0 && flags="$flags MACOSX_DEPLOYMENT_TARGET=14.0"
# Linux: imap-send uses curl; no OpenSSL to ship.
[ "$(uname)" = Linux ] && flags="$flags NO_OPENSSL=YesPlease"
# Clang ignores Rosetta, so Intel builds name their architecture.
compiler=()
[ "$(uname)" != Darwin ] || compiler=("CC=cc -arch $(uname -m)")
record="$flags${compiler[@]+ ${compiler[*]}}"
if [ "$(cat "$out/.flags" 2>/dev/null)" != "$record" ]; then
  rm -rf "$out"
fi
# shellcheck disable=SC2086
env -u CARGO_TARGET_DIR make -C "$src" $flags ${compiler[@]+"${compiler[@]}"} -j"$jobs" install # Git's Rust uses its target/
echo "$record" >"$out/.flags"
"$out/bin/git" --version
