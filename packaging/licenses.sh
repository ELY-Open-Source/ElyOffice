#!/usr/bin/env bash
# Writes an app's licences and notices. Usage: licenses.sh <slug> <dir>
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
slug=${1:?app slug}
out=${2:?licence folder}
put() { # into, files...
  local into=$out/$1
  shift
  mkdir -p "$into"
  for file in "$@"; do
    [ -s "$file" ] || {
      echo "licenses.sh: no $file; the release cannot ship without it" >&2
      exit 1
    }
    cp "$file" "$into/"
  done
}
put . "$root/LICENSE"
case $slug in
  ely-doc | ely-table | ely-show) crates=(ely-lok) ;;
  ely-pdf | ely-mark) crates=(ely-pdf-engine) ;;
  *) crates=() ;;
esac
python3 "$root/packaging/crates.py" "$slug" ${crates[@]+"${crates[@]}"} >"$out/rust-crates.txt"
case $(uname) in
  Darwin) lo=${ELY_LO_APP:-$root/engines/lo-src/instdir/LibreOffice.app}/Contents/Resources ;;
  *) lo=${ELY_LO_DIR:-$root/engines/lo-src/instdir} ;;
esac
# Windows builds name LibreOffice's licence license.txt.
lo_licence=$lo/LICENSE
[ -e "$lo_licence" ] || lo_licence=$lo/license.txt
case $slug in
  ely-doc | ely-table | ely-show)
    put LibreOffice "$lo_licence" "$lo/LICENSE.html" "$lo/NOTICE" "$lo/CREDITS.fodt"
    ;;
  ely-pdf | ely-mark)
    put MuPDF "$root/engines/mupdf-src/COPYING"
    # Every third party the MuPDF build compiled in.
    case $(uname) in
      MINGW* | MSYS*)
        parts=$(python3 "$root/packaging/windows/mupdf-parts.py" "$root/engines/mupdf-src" parts)
        families=$(python3 "$root/packaging/windows/mupdf-parts.py" "$root/engines/mupdf-src" fonts)
        ;;
      Darwin)
        parts=$(ls "$root/engines/mupdf-build/thirdparty")
        families=$(ls "$root/engines/mupdf-build/generated/resources/fonts")
        ;;
      *)
        # Linux links the font files in as objects.
        parts=$(ls "$root/engines/mupdf-build/thirdparty")
        families=$(ls "$root/engines/mupdf-build/resources/fonts")
        ;;
    esac
    for name in $parts; do
      found=$(find "$root/engines/mupdf-src/thirdparty/$name" -maxdepth 1 -type f \
        \( -iname 'licen[cs]e*' -o -iname 'copying*' -o -iname '*-license.txt' \))
      [ "$name" = libjpeg ] && found=$root/engines/mupdf-src/thirdparty/libjpeg/README
      [ -n "$found" ] || { echo "licenses.sh: no licence for MuPDF's $name" >&2; exit 1; }
      # shellcheck disable=SC2086
      put "MuPDF/$name" $found
    done
    # Every font family MuPDF built into the worker.
    for name in $families; do
      fonts=$root/engines/mupdf-src/resources/fonts/$name
      found=$(find "$fonts" -maxdepth 1 -type f \( -iname 'licen[cs]e*' -o -iname 'copying*' -o -iname 'notice*' -o -iname 'ofl*' \))
      [ -n "$found" ] || { echo "licenses.sh: no licence for MuPDF's $name fonts" >&2; exit 1; }
      # A README tells the fonts' source; never the licence.
      found="$found $(find "$fonts" -maxdepth 1 -type f -iname 'readme*')"
      # shellcheck disable=SC2086
      put "MuPDF/fonts/$name" $found
    done
    case $(uname) in
      MINGW* | MSYS*) put OpenSSL "$root/engines/vcpkg/installed/x64-windows-static/share/openssl/copyright" ;;
      *) put OpenSSL "$(pkg-config --variable=prefix libcrypto)/LICENSE.txt" ;;
    esac
    put GreatVibes "$root/crates/ely-pdf-engine/fonts/OFL.txt"
    ;;
esac
case $slug in
  ely-pdf) put tessdata "${ELY_TESSDATA:-$root/engines/tessdata}/LICENSE" ;;
  ely-mark)
    # RaTeX embeds KaTeX's fonts.
    katex=$(cargo metadata --format-version 1 --locked --manifest-path "$root/Cargo.toml" |
      python3 -c 'import json, pathlib, sys; print(next(pathlib.Path(p["manifest_path"]).parent for p in json.load(sys.stdin)["packages"] if p["name"] == "ratex-katex-fonts"))')
    put KaTeX "$katex/fonts/OFL.txt" "$katex/fonts/FONT_NOTICE.txt"
    ;;
esac
# ElyTree carries Git; on Linux ElyMark does too.
if [ "$slug" = ely-tree ] || { [ "$slug" = ely-mark ] && [ "$(uname)" = Linux ]; }; then
  case $(uname) in
    MINGW* | MSYS*) put Git "$root/engines/git/LICENSE.txt" ;;
    *) put Git "$root/engines/git-src/COPYING" "$root/engines/git-src/LGPL-2.1" ;;
  esac
  put git-lfs "$root/engines/git/share/doc/git-lfs/LICENSE.md"
fi
