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
# A tool whose first version number is at least `min`.
least() { # name, min, then the command printing its version
  local name=$1 min=$2 out have
  shift 2
  if ! out=$("$@" 2>&1); then
    echo "MISSING $name (README, Build): ${out%%$'\n'*}"
    failed=1
    return
  fi
  have=$(grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' <<<"$out" | head -1)
  if [ -z "$have" ] || [ "$(printf '%s\n%s\n' "$min" "$have" | sort -V | head -1)" != "$min" ]; then
    echo "OLD     $name ${have:-unknown}, want $min or newer (README, Build)"
    failed=1
    return
  fi
  printf '%-10s %s\n' "$name" "${out%%$'\n'*}"
}

check rustc --version
check cargo --version
check git --version
check python3 --version
check curl --version
# The site's Vite and Wrangler.
least node 22.12 node --version
check pnpm --version

case $(uname -s) in
  Darwin)
    check clang --version
    check pkg-config --version
    check cmake --version
    check ninja --version
    need metal xcrun -sdk macosx metal --version
    # What build-libreoffice.sh puts first on PATH.
    for formula in gnu-sed bison flex gettext; do
      need "$formula" brew list --versions "$formula"
    done
    least make 4.2 "$(brew --prefix make 2>/dev/null)/libexec/gnubin/make" --version
    check autoconf --version
    check aclocal --version
    ;;
  MINGW* | MSYS* | CYGWIN*)
    check cmake --version
    check ninja --version
    vswhere="$(cygpath -u "$PROGRAMFILES") (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
    # LibreOffice builds with Visual Studio 2022 alone.
    msvc() {
      local found
      found=$("$vswhere" -products '*' -version '[17.0,18.0)' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath) &&
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
    # LibreOffice builds under Cygwin, with its own make.
    cygwin=${ELY_CYGWIN:-/c/cygwin64}
    least lo-make 4.2 "$cygwin/opt/lo/bin/make.exe" --version
    need autoconf "$cygwin/bin/bash.exe" -lc 'autoconf --version'
    need aclocal "$cygwin/bin/bash.exe" -lc 'aclocal --version'
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
