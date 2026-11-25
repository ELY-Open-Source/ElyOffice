#!/usr/bin/env bash
# Usage: build-libreoffice.sh [--symbols] — the pinned, patched LibreOffice.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
src=$root/engines/lo-src
symbols=
case ${1:-} in
  "") ;;
  # A kernel with symbols for sampling; never shipped.
  --symbols) src=$root/engines/lo-sym symbols=1 ;;
  *)
    echo "build-libreoffice.sh: unknown option $1" >&2
    exit 2
    ;;
esac
pin=$(awk '$1 == "fetch" && $2 == "libreoffice-core" { print $4 }' "$root/scripts/fetch-references.sh")
options=(
  --enable-bogus-pkg-config
  --enable-mergelibs
  --enable-release-build
  --without-java
  --without-junit
  --without-help
  --disable-odk
  --disable-online-update
  --disable-extension-integration
  --disable-scripting-beanshell
  --disable-scripting-javascript
  --disable-firebird-sdbc
  --disable-postgresql-sdbc
  --disable-ext-nlpsolver
  --disable-ext-wiki-publisher
  --disable-report-builder
  --disable-sdremote
  --disable-sdremote-bluetooth
  --disable-poppler
  --disable-lotuswordpro
  --disable-gpgmepp
  --disable-cups
  --disable-evolution2
  --with-galleries=no
  --with-lang=
  --with-theme=colibre
  --with-fonts
  --disable-python
  --disable-coinmp
  --disable-lpsolve
  --without-doxygen
  --with-myspell-dicts
)
[ -z "$symbols" ] || options+=(--enable-symbols)

case $(uname) in
  Darwin)
    app=$src/instdir/LibreOffice.app/Contents/Resources
    jobs=$(sysctl -n hw.ncpu)
    ;;
  Linux)
    # Collabora's LOKit set: headless, bundled libraries.
    options+=(
      --disable-gui
      --disable-dbus
      --disable-gio
      --disable-dconf
      --disable-randr
      --disable-gstreamer-1-0
      --without-system-cairo
      --without-system-fontconfig
      --without-system-freetype
      --without-system-graphite
      --without-system-harfbuzz
      --without-system-openssl
      --without-system-libpng
      --without-system-libxml
      --without-system-jpeg
      --without-system-expat
      --without-system-curl
      --without-system-icu
      --without-system-nss
      --without-system-zlib
    )
    app=$src/instdir/share
    jobs=$(nproc)
    ;;
  CYGWIN*)
    # Unix-only switches; configure refuses them here.
    kept=()
    for option in "${options[@]}"; do
      case $option in
        --enable-bogus-pkg-config | --disable-cups | --disable-evolution2 | --disable-python) ;;
        *) kept+=("$option") ;;
      esac
    done
    options=("${kept[@]}" --with-visual-studio=2022 --disable-ccache --disable-cli --without-dotnet)
    # Cygwin's own make, never Git Bash's; MSYSTEM misleads the build.
    unset MSYSTEM MSYSTEM_PREFIX MINGW_PREFIX MSYSTEM_CHOST MSYSTEM_CARCH
    PATH=/opt/lo/bin:$PATH
    app=$src/instdir/share
    jobs=$(nproc)
    ;;
  *)
    echo "build-libreoffice.sh: no build for $(uname)" >&2
    exit 1
    ;;
esac
if [ ! -d "$src/.git" ]; then
  git init -q "$src"
  git -C "$src" remote add origin https://github.com/LibreOffice/core.git
  git -C "$src" fetch -q --depth 1 origin "$pin"
  git -C "$src" checkout -q FETCH_HEAD
fi
# The bundled spelling dictionaries (DOC-12).
git -C "$src" config submodule.dictionaries.url https://github.com/LibreOffice/dictionaries.git
git -C "$src" submodule update -q --init --depth 1 dictionaries
[ "$(git -C "$src" rev-parse HEAD)" = "$pin" ] || {
  echo "build-libreoffice.sh: $src is not at $pin" >&2
  exit 1
}
# A moved tree keeps old paths; make fails there.
"$root/scripts/lo-configured.sh" "$src"
"$root/scripts/apply-lo-patches.sh" "$src"

if [ "$(uname)" = Darwin ]; then
  for tool in make gnu-sed bison flex gettext; do
    prefix=$(brew --prefix "$tool")
    PATH=$prefix/libexec/gnubin:$prefix/bin:$PATH
  done
  export PATH PKG_CONFIG_LIBDIR=/var/empty PKG_CONFIG_PATH=
fi

cd "$src"
# A fresh tree has no configuration to read.
sdk=
[ ! -f config_host.mk ] || sdk=$(sed -n 's/^export MACOSX_SDK_PATH=//p' config_host.mk)
if [ -n "$sdk" ] && [ ! -d "$sdk" ]; then
  echo "build-libreoffice.sh: configured SDK $sdk is gone; reconfiguring" >&2
  rm config_host.mk
fi
if [ ! -f config_host.mk ] || ! printf '%s\n' "${options[@]}" | cmp -s - autogen.lastrun; then
  ./autogen.sh "${options[@]}"
fi
make -j"$jobs"

# A bare make brings back what this prunes.
"$root/scripts/lo-extras.sh" "$app/.." | while IFS= read -r extra; do rm -rf "$extra"; done
extensions=$app/extensions
echo "build-libreoffice.sh: dictionaries kept: $(ls "$extensions" | tr '\n' ' ')"
en=$extensions/dict-en
python3 - "$en/META-INF/manifest.xml" <<'PY'
import re, sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
gone = r'\s*<manifest:file-entry[^>]*?"(?:Lightproof\.components|Linguistic\.xcu|dialog/[^"]*)"[^>]*/>'
kept = re.sub(gone, "", text, flags=re.S)
for left in ("Lightproof", "Linguistic.xcu", "dialog/"):
    if left in kept:
        sys.exit(f"build-libreoffice.sh: {left} is still in {path}")
open(path, "w", encoding="utf-8").write(kept)
PY
# Ely's kernel defaults: Excel A1 formulas.
cp "$root/packaging/libreoffice/ely.xcd" "$app/registry/"
# Fonts Word files ask for that LibreOffice lacks.
"$root/scripts/fetch-fonts.sh"
mkdir -p "$app/fonts/truetype"
tail -n +2 "$root/packaging/libreoffice/fonts/SOURCES.tsv" | while IFS=$'\t' read -r folder file _; do
  cp "$root/engines/fonts/$folder/$file" "$app/fonts/truetype/"
done
# Linux: Ely's font aliases join fontconfig's local file.
if [ "$(uname)" = Linux ]; then
  conf="$app/fonts/truetype/fc_local.conf"
  [ -f "$conf" ] || {
    echo "build-libreoffice.sh: no $conf to add the font aliases to" >&2
    exit 1
  }
  python3 - "$conf" "$root/packaging/libreoffice/ely-fonts.conf" <<'PY'
import sys
conf, extra = sys.argv[1:]
text = open(conf, encoding="utf-8").read()
aliases = open(extra, encoding="utf-8").read()
tail = text.rindex("</fontconfig>")
# A rebuild replaces the aliases an earlier build added.
start = text.find("\n" + aliases.splitlines()[0])
head = text[:start] if start >= 0 else text[:tail]
open(conf, "w", encoding="utf-8").write(head + "\n" + aliases + text[tail:])
PY
fi

if [ "$(uname)" = Linux ]; then
  # Meson leaves build-tree RUNPATHs in copied libraries, as cairo's.
  built=$(cd "$src" && pwd -P)
  files=$(mktemp)
  find "$src/instdir" -type f -print0 >"$files" || {
    echo "build-libreoffice.sh: cannot list $src/instdir" >&2
    exit 1
  }
  while IFS= read -r -d '' lib; do
    magic=$(head -c 4 "$lib" | od -An -c | tr -d ' ') || exit 1
    [ "$magic" = '177ELF' ] || continue
    dynamic=$(readelf -dW "$lib") || exit 1
    paths=$(sed -n 's/.*(R\(UN\)\{0,1\}PATH).*\[\(.*\)\]/\2/p' <<<"$dynamic")
    # Configure records the path with links resolved.
    if [[ $paths == *"$built"* ]]; then
      # shellcheck disable=SC2016
      patchelf --set-rpath '$ORIGIN' "$lib"
      echo "build-libreoffice.sh: $lib now loads from \$ORIGIN"
    fi
  done <"$files"
  rm "$files"
fi
if [ "$(uname)" = Darwin ]; then
  "$root/scripts/inert-lo.sh" "$src/instdir/LibreOffice.app"
fi
# The id the engine and the app agree on.
case $(uname) in
  Darwin) "$root/scripts/kernel-id.sh" "$src/instdir/LibreOffice.app/Contents" "$src/instdir/LibreOffice.app/Contents/Resources/ely-kernel-id" ;;
  *) "$root/scripts/kernel-id.sh" "$src/instdir" "$src/instdir/program/ely-kernel-id" ;;
esac
