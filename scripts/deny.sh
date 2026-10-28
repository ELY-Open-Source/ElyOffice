#!/usr/bin/env bash
# cargo-deny; a skip no longer needed fails too.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}
shift || true
cargo deny --manifest-path "$root/Cargo.toml" --workspace --all-features check \
  -D unnecessary-skip -D unmatched-skip "$@"
