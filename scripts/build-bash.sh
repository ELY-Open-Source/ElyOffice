#!/usr/bin/env bash
# Pinned GNU bash, x86_64, for the Intel release under Rosetta.
set -euo pipefail

out=${1:-$HOME/ely-share/bash-x86_64}
version=5.3
# Checked once against GNU's signature by Chet Ramey.
sum=0d5cd86965f869a26cf64f4b71be7b96f90a3ba8b3d74e27e8e9d9d5550f31ba
[ "$(uname -s)" = Darwin ] || {
  echo "build-bash.sh: macOS only" >&2
  exit 1
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL -o "$work/bash.tar.gz" "https://ftp.gnu.org/gnu/bash/bash-$version.tar.gz"
got=$(shasum -a 256 "$work/bash.tar.gz" | cut -d' ' -f1)
[ "$got" = "$sum" ] || {
  echo "build-bash.sh: bash-$version.tar.gz hash $got, want $sum" >&2
  exit 1
}
tar -xzf "$work/bash.tar.gz" -C "$work"
cd "$work/bash-$version"
arch -x86_64 ./configure --prefix="$out" CC="clang -arch x86_64"
arch -x86_64 make -j"$(sysctl -n hw.ncpu)"
rm -rf "$out"
arch -x86_64 make install
built=$(lipo -archs "$out/bin/bash")
[ "$built" = x86_64 ] || {
  echo "build-bash.sh: $out/bin/bash is $built, want x86_64" >&2
  exit 1
}
echo "build-bash.sh: $(arch -x86_64 "$out/bin/bash" --version | head -n 1) at $out/bin/bash"
