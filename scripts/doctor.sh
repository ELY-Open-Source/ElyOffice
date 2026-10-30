#!/usr/bin/env bash
# Tools and system packages; exit 1 if any lack.
set -uo pipefail

failed=0
check() { # tool version-args...
  local tool=$1 out
  shift
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "MISSING $tool"
    failed=1
  elif ! out=$("$tool" "$@" 2>&1); then
    echo "BROKEN  $tool: ${out%%$'\n'*}"
    failed=1
  else
    printf '%-10s %s\n' "$tool" "${out%%$'\n'*}"
  fi
}
# What the build needs, beyond tools; README lists the packages.
need() { # name, then the command that proves it
  local name=$1 out
  shift
  if out=$("$@" 2>&1); then
    printf '%-10s %s\n' "$name" "${out%%$'\n'*}"
  else
    echo "MISSING $name (README, Build): ${out%%$'\n'*}"
    failed=1
  fi
}

check rustc --version
check cargo --version
check git --version
check python3 --version
check curl --version
check node --version
check pnpm --version

case $(uname -s) in
  Darwin)
    check clang --version
    check make --version
    check pkg-config --version
    check cmake --version
    check ninja --version
    need metal xcrun -sdk macosx metal --version
    ;;
  MINGW* | MSYS* | CYGWIN*)
    check cmake --version
    check ninja --version
    vswhere="$(cygpath -u "$PROGRAMFILES") (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    msvc() {
      local found
      found=$("$vswhere" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) &&
        [ -n "$found" ] && echo "$found"
    }
    # As gpui's build script looks: variable, PATH, SDK.
    fxc() {
      local key='HKLM\SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0' folder
      if [ -n "${GPUI_FXC_PATH:-}" ] && [ -e "$GPUI_FXC_PATH" ]; then
        echo "$GPUI_FXC_PATH"
        return
      fi
      where.exe fxc.exe 2>/dev/null | head -1 | grep . && return
      folder=$(MSYS2_ARG_CONV_EXCL='*' reg query "$key" /v InstallationFolder | awk -F'REG_SZ' 'NF > 1 { sub(/^ +/, "", $2); print $2 }' | tr -d '\r') &&
        compgen -G "$(cygpath -u "$folder")bin/*/x64/fxc.exe" | tail -1
    }
    need msvc msvc
    need fxc fxc
    ;;
  Linux)
    check cc --version
    check make --version
    check pkg-config --version
    check docker --version
    for lib in dbus-1 fontconfig xcb xkbcommon xkbcommon-x11; do
      need "$lib" pkg-config --modversion "$lib"
    done
    ;;
esac

exit "$failed"
