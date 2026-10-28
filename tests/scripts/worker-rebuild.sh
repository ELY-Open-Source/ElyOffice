#!/usr/bin/env bash
# A shim edit rebuilds the worker's native code, not just the stamp.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
build() { cargo build -v -p ely-pdf-worker --manifest-path "$root/Cargo.toml" 2>&1; }
build >/dev/null
for file in ely_pages.c ely_shim.h; do
  # Times only: the files' contents stay as they are.
  touch "$root/crates/ely-pdf-worker/shim/$file"
  said=$(build)
  if ! grep -Eq 'Running `[^`]*build[/\\]ely-pdf-worker-[0-9a-f]+[/\\]build-script-build' <<<"$said"; then
    echo "FAIL: shim/$file changed and the worker's build script did not run"
    exit 1
  fi
done
echo "PASS: a changed shim rebuilds the worker's native code"
