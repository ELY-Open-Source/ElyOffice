#!/usr/bin/env bash
# Pinned upstream sources. Read-only. Every entry names one commit.
set -euo pipefail

ref="$(cd "$(dirname "$0")/.." && pwd)/reference"
mkdir -p "$ref"

sparse_ok() { # dir [sparse paths...]
  local dir=$1
  shift
  [ "$(git -C "$dir" sparse-checkout list 2>/dev/null | sort)" = "$(printf '%s\n' "$@" | sort)" ]
}

fetch() { # name url rev [sparse paths...]
  local name=$1 url=$2 rev=$3
  shift 3
  local dir="$ref/$name"
  if [ -d "$dir/.git" ] && [ "$(git -C "$dir" rev-parse HEAD)" = "$rev" ] && sparse_ok "$dir" "$@"; then
    echo "ok    $name $rev"
    return
  fi
  rm -rf "$dir"
  git init -q "$dir"
  # Upstream bytes, whatever the host's line-ending setting.
  git -C "$dir" config core.autocrlf false
  git -C "$dir" config core.eol lf
  git -C "$dir" remote add origin "$url"
  if [ $# -gt 0 ]; then
    git -C "$dir" sparse-checkout init --cone
    git -C "$dir" sparse-checkout set "$@"
    git -C "$dir" fetch -q --depth 1 --filter=blob:none origin "$rev"
  else
    git -C "$dir" fetch -q --depth 1 origin "$rev"
  fi
  git -C "$dir" checkout -q FETCH_HEAD
  echo "fetch $name $rev"
}

fetch Ely-GPUI-Components https://github.com/ELY-Open-Source/Ely-GPUI-Components.git da28586001aad53525168a75747a9308230949cc
fetch zed https://github.com/zed-industries/zed.git 1a28cff4b409169bac058bca40dfbfeb7621d19b
fetch libreoffice-core https://github.com/LibreOffice/core.git 8221e31b3ac356a1623c672912a3d2b492f7e3d1 \
  include/LibreOfficeKit libreofficekit desktop/source/lib desktop/inc \
  officecfg/registry/data/org/openoffice/Office/UI
fetch mupdf https://github.com/ArtifexSoftware/mupdf.git 8ad45e92f0935d3d87f1db3f873086472a5e1b24
git -C "$ref/mupdf" submodule update --init --depth 1 --recursive
fetch git https://github.com/git/git.git a018953688f1b10bddf91bff8747068f5f4746a4
fetch tesseract https://github.com/tesseract-ocr/tesseract.git db0ec62f81b0737fbbe184d8fea40af5738f8eef
fetch marktext https://github.com/marktext/marktext.git b29cb5621e12cedb801c1cd886d9005159feaa2e
fetch colamd https://github.com/marswaveai/ColaMD.git 5e0f3c3584bb771180afe1a4670efbaf28dee65b
fetch magpie https://github.com/yetone/magpie.git e4c8316e3fa9d73683079df3f9142561265bc1bb
fetch agent-client-protocol https://github.com/agentclientprotocol/agent-client-protocol.git 9e032156545412be9bba5e092f12d0080c499b6d
fetch markion https://github.com/willmove/markion.git 2bd3a89fe39ec1f07bc02e33ae21601fe8b977c6
fetch markdown-rs https://github.com/wooorm/markdown-rs.git 1506572f9b406431402928f3a8b3df0b4ae2d8f5
fetch omarchy https://github.com/basecamp/omarchy.git f45461a38f2b0a12ba39837eb41b08c012051780 bin themes default/themed
