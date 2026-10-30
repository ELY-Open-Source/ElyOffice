#!/usr/bin/env bash
# The release gate: every test, engines required; ci.sh calls it.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
export ELY_REQUIRE_ENGINES=1
# No incremental cache: repeated runs would fill the disk.
export CARGO_INCREMENTAL=0
# Linux's MuPDF links the pinned static OpenSSL.
[ "$(uname -s)" != Linux ] || export PKG_CONFIG_PATH=$root/engines/openssl/lib/pkgconfig
# Tests run the workers; they never rebuild them.
cargo build -p ely-lok --bins -p ely-pdf-worker
cargo test --workspace
