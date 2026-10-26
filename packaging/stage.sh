#!/usr/bin/env bash
# Stages an app with its engines. Usage: stage.sh <slug> <dir>
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
slug=${1:?app slug, such as ely-doc}
out=${2:?the folder to fill}
case $(uname) in
  MINGW* | MSYS*) exe=.exe ;;
  Linux) exe= ;;
  *)
    echo "stage.sh: Windows or Linux only" >&2
    exit 1
    ;;
esac
grep -q "^$slug	" "$root/packaging/apps.tsv" || {
  echo "stage.sh: unknown app $slug" >&2
  exit 1
}
uses() { grep -q "^$1" "$root/apps/$slug/Cargo.toml"; }
need() { # path, what to run
  [ -e "$1" ] || {
    echo "stage.sh: no $1; run $2" >&2
    exit 1
  }
}
build() { cargo build --manifest-path "$root/Cargo.toml" --release "$@"; }
release=${CARGO_TARGET_DIR:-$root/target}/release

rm -rf "$out"
mkdir -p "$out"
build -p "$(cargo pkgid --manifest-path "$root/apps/$slug/Cargo.toml")"
cp "$release/$slug$exe" "$out/"
if uses ely-lok; then
  lo=${ELY_LO_DIR:-$root/engines/lo-src/instdir}
  need "$lo/program" scripts/build-libreoffice.sh
  build -p ely-lok --bin ely-lok-worker
  cp "$release/ely-lok-worker$exe" "$out/"
  # The kernel in LibreOffice's layout, beside the app.
  mkdir "$out/libreoffice"
  cp -R "$lo/program" "$lo/share" "$lo/presets" "$out/libreoffice/"
  cp "$lo/LICENSE.html" "$lo/NOTICE" "$lo/CREDITS.fodt" "$out/libreoffice/"
  # Merged-library builds leave text placeholders.
  while IFS= read -r stub; do
    if [ "$(cat "$stub")" = "invalid - merged lib" ]; then
      rm "$stub"
    fi
  done < <(find "$out/libreoffice/program" -type f -size -64c)
  if [ -n "$exe" ]; then
    # The kernel needs the VC++ runtime; it ships beside it.
    crt=$(ls -d "/c/Program Files/Microsoft Visual Studio/2022/"*/VC/Redist/MSVC/*/x64/Microsoft.VC143.CRT | tail -n 1)
    need "$crt" "the Visual Studio 2022 C++ tools"
    cp "$crt"/*.dll "$out/libreoffice/program/"
  fi
fi
if uses ely-pdf-engine; then
  build -p ely-pdf-engine --bin ely-pdf-worker
  cp "$release/ely-pdf-worker$exe" "$out/"
fi
# OCR's languages: ElyPDF alone reads text from pictures.
if [ "$slug" = ely-pdf ]; then
  tessdata=${ELY_TESSDATA:-$root/engines/tessdata}
  need "$tessdata/eng.traineddata" scripts/fetch-tessdata.sh
  mkdir "$out/tessdata"
  cp "$tessdata"/{eng,chi_sim,chi_tra}.traineddata "$out/tessdata/"
fi
# ElyTree carries Git; so does Linux ElyMark's standalone AppImage.
if [ "$slug" = ely-tree ] || { [ "$slug" = ely-mark ] && [ "$(uname)" = Linux ]; }; then
  need "$root/engines/git" "scripts/build-git.sh and scripts/fetch-git-lfs.sh"
  cp -R "$root/engines/git" "$out/git"
fi
"$root/packaging/licenses.sh" "$slug" "$out/licenses"
echo "$out"
