#!/usr/bin/env bash
# Usage: lo-extras.sh <kernel> — lists bundled extras the build prunes.
set -euo pipefail
kernel=${1:?the kernel folder, such as engines/lo-src/instdir}
# share/ on Linux and Windows, Resources/ in a macOS bundle.
for extensions in "$kernel"/share/extensions "$kernel"/Resources/extensions; do
  [ -d "$extensions" ] || continue
  # English only: each dictionary slows LOK start (PERF-01).
  for dict in "$extensions"/dict-*; do
    [ -e "$dict" ] && [ "${dict##*/}" != dict-en ] && echo "$dict"
  done
  # Lightproof is Python: inert without it, a hang on Windows.
  for part in Lightproof.py Lightproof.components Linguistic.xcu pythonpath dialog; do
    [ ! -e "$extensions/dict-en/$part" ] || echo "$extensions/dict-en/$part"
  done
done
exit 0
