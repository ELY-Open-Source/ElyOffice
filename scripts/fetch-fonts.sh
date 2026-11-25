#!/usr/bin/env bash
# Fonts the kernel ships, from upstream, checked by SHA-256.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
list=$root/packaging/libreoffice/fonts/SOURCES.tsv
out=$root/engines/fonts
# Upstream files named by their hash; source.sh packs them.
cache=$out/.upstream
mkdir -p "$cache"
sum() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
fail() {
  echo "fetch-fonts.sh: $*" >&2
  exit 1
}

upstream() { # url hash: leaves a checked copy at $cache/<hash>
  local url=$1 hash=$2 saved=$cache/$2 got
  if [ -f "$saved" ] && [ "$(sum "$saved")" = "$hash" ]; then
    return
  fi
  rm -f "$saved"
  local into=$saved.part
  # Cygwin finds Windows' curl, which takes C:/ paths only.
  case $(uname) in
    CYGWIN*) into=$(cygpath -m "$into") ;;
  esac
  curl -fsSL -o "$into" "$url" || {
    rm -f "$saved.part"
    fail "cannot fetch $url"
  }
  got=$(sum "$saved.part")
  [ "$got" = "$hash" ] || {
    rm -f "$saved.part"
    fail "$url is $got, not $hash"
  }
  mv "$saved.part" "$saved"
}

while IFS=$'\t' read -r folder file url hash whole; do
  dest=$out/$folder/$file
  if [ -f "$dest" ] && [ "$(sum "$dest")" = "$hash" ]; then
    continue
  fi
  source=${url%%#*}
  upstream "$source" "$whole"
  mkdir -p "$out/$folder"
  if [ "$source" != "$url" ]; then
    unzip -p "$cache/$whole" "${url#*#}" >"$dest.part" 2>/dev/null || {
      rm -f "$dest.part"
      fail "$source has no ${url#*#}"
    }
  else
    cp "$cache/$whole" "$dest.part"
  fi
  got=$(sum "$dest.part")
  [ "$got" = "$hash" ] || {
    rm -f "$dest.part"
    fail "$folder/$file is $got, not $hash"
  }
  mv "$dest.part" "$dest"
  echo "ok    $folder/$file"
done < <(tail -n +2 "$list")
