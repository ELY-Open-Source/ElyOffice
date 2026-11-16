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
# Test programs may sit apart from the workers: name them.
bins=$(cargo metadata --no-deps --format-version 1 |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])' | tr -d '\r')/debug
exe=
case $(uname -s) in
  MINGW* | MSYS*) exe=.exe ;;
esac
export ELY_LOK_WORKER=$bins/ely-lok-worker$exe ELY_PDF_WORKER=$bins/ely-pdf-worker$exe
cargo test --workspace

# Idle engines: ten minutes, no CPU, memory as recorded.
cargo build -p ely-doc -p ely-table -p ely-show
idle=$(mktemp -d)
case $(uname -s) in
  MINGW* | MSYS*) idle=$(cygpath -w "$idle") ;;
esac
stop() { for slug in ely-doc ely-table ely-show; do ELY_DATA_ROOT=$idle "$bins/$slug$exe" engine stop; done; }
trap stop EXIT
for slug in ely-doc ely-table ely-show; do ELY_DATA_ROOT=$idle "$bins/$slug$exe" engine; done
python3 tests/perf/idle.py "$idle"
