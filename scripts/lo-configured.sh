#!/usr/bin/env bash
# Usage: lo-configured.sh <lo-src> — fails if configured at another path.
set -euo pipefail
src=${1:?usage: lo-configured.sh <lo-src>}
config=$src/config_host.mk
# Not configured yet: the build configures it here.
[ -f "$config" ] || exit 0
was=$(sed -n 's/^export SRCDIR=//p' "$config")
here=$(cd "$src" && pwd -P)
case $(uname) in
  CYGWIN*) here=$(cygpath -m "$here") ;;
esac
[ "$was" = "$here" ] || {
  echo "lo-configured.sh: $src was configured at '$was', not '$here'; remove its config_host.mk, workdir and instdir, then build again" >&2
  exit 1
}
