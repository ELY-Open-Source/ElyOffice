#!/usr/bin/env bash
# Pinned static OpenSSL: Intel Macs, and Linux for MuPDF.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=$root/engines/openssl
version=4.0.3
sum=325b5c806167c13b40b1ffeadfe0248197c00eccc4cf123ec1e28d2d2fd216d9
arch=$(uname -m)
case "$(uname -s)-$arch" in
  Darwin-x86_64) target=darwin64-x86_64-cc ;;
  Darwin-arm64) target=darwin64-arm64-cc ;;
  Linux-x86_64) target=linux-x86_64 ;;
  Linux-aarch64) target=linux-aarch64 ;;
  *)
    echo "build-openssl.sh: no build for $(uname -s)-$arch" >&2
    exit 1
    ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL -o "$work/openssl.tar.gz" "https://github.com/openssl/openssl/releases/download/openssl-$version/openssl-$version.tar.gz"
got=$(shasum -a 256 "$work/openssl.tar.gz" | cut -d' ' -f1)
[ "$got" = "$sum" ] || {
  echo "build-openssl.sh: openssl-$version.tar.gz hash $got, want $sum" >&2
  exit 1
}
tar -xzf "$work/openssl.tar.gz" -C "$work"
# The bundles say macOS 14; OpenSSL must not need newer.
export MACOSX_DEPLOYMENT_TARGET=14.0
cd "$work/openssl-$version"
./Configure "$target" no-shared no-tests --prefix="$out" --libdir=lib
make -j"$(sysctl -n hw.ncpu 2>/dev/null || nproc)"
rm -rf "$out"
make install_sw
# Packaging ships the licence from this prefix.
cp LICENSE.txt "$out/"
if [ "$(uname)" = Darwin ]; then
  built=$(lipo -archs "$out/lib/libcrypto.a")
  [ "$built" = "$arch" ] || {
    echo "build-openssl.sh: libcrypto.a is $built, want $arch" >&2
    exit 1
  }
fi
echo "build-openssl.sh: $("$out/bin/openssl" version) for $arch; PKG_CONFIG_PATH=$out/lib/pkgconfig"
