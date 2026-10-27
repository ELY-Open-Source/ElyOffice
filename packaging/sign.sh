#!/usr/bin/env bash
# sign.sh <manifest>...: writes each one's .sig with ELY_UPDATE_KEY.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
: "${ELY_UPDATE_KEY:?the seed of the release key, base64}"
cargo run --quiet --locked --manifest-path "$root/Cargo.toml" --bin sign-manifest -- "$@"
