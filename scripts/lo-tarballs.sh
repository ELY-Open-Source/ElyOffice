#!/usr/bin/env bash
# Usage: lo-tarballs.sh <lo-src>; prints name, sha256, URL it fetches.
set -euo pipefail

src=$(cd "${1:?a configured LibreOffice tree}" && pwd)
[ -f "$src/config_host.mk" ] || {
  echo "lo-tarballs.sh: $src is not configured; run scripts/build-libreoffice.sh" >&2
  exit 1
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# config_host.mk on Windows holds C:/ paths.
case $(uname) in
  CYGWIN*)
    src=$(cygpath -m "$src")
    work=$(cygpath -m "$work")
    ;;
esac
# Each fetch prints its line instead of downloading.
cat >"$work/list.mk" <<'EOF'
include $(BUILDDIR)/config_host.mk
include $(SRCDIR)/download.lst
include $(SRCDIR)/Makefile.fetch
define fetch__Download_item
@printf '%s\t%s\t%s\n' '$2' '$3' '$1/$2' >>$(LIST)

endef
EOF
: >"$work/list.tsv"
# The command-line PATH beats the one config_host.mk exports.
make -s -f "$work/list.mk" PATH="$PATH" gb_Side=host BUILDDIR="$src" SRCDIR="$src" \
  WORKDIR="$work/work" TARFILE_LOCATION="$work/tar" LIST="$work/list.tsv" "$work/work/download" >&2
LC_ALL=C sort "$work/list.tsv"
