#!/usr/bin/env bash
# Builds the pinned MuPDF, with patches/mupdf, into engines/mupdf-build.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
src=$root/engines/mupdf-src
out=$root/engines/mupdf-build
jobs=$(sysctl -n hw.ncpu 2>/dev/null || nproc)

# reference/ stays read-only; the copy takes the patches.
mkdir -p "$src"
case $(uname -s) in
  MINGW* | MSYS*)
    rm -rf "$src" && mkdir -p "$src"
    tar -C "$root/reference/mupdf" --exclude .git -cf - . | tar -C "$src" -xf -
    ;;
  *) rsync -a --delete --exclude .git "$root/reference/mupdf/" "$src/" ;;
esac
for patch in "$root"/patches/mupdf/*.patch; do
  patch -d "$src" -p1 --forward --batch --quiet <"$patch"
done
case $(uname -s) in
  MINGW* | MSYS*)
    # MSVC: the static runtime Rust links, OpenSSL from vcpkg.
    ssl=$(cygpath -w "$root/engines/vcpkg/installed/x64-windows-static")
    [ -f "$ssl/lib/libcrypto.lib" ] || {
      echo "build-mupdf.sh: no static OpenSSL; run engines/vcpkg/vcpkg install openssl:x64-windows-static" >&2
      exit 1
    }
    cat >"$src/ely.props" <<PROPS
<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <ItemDefinitionGroup>
    <ClCompile>
      <RuntimeLibrary>MultiThreaded</RuntimeLibrary>
      <PreprocessorDefinitions>HAVE_LIBCRYPTO;%(PreprocessorDefinitions)</PreprocessorDefinitions>
      <AdditionalIncludeDirectories>$ssl\include;%(AdditionalIncludeDirectories)</AdditionalIncludeDirectories>
    </ClCompile>
  </ItemDefinitionGroup>
</Project>
PROPS
    msbuild=$("/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe" -latest -find 'MSBuild\**\Bin\MSBuild.exe' | tr -d '\r')
    "$msbuild" "$(cygpath -w "$src/platform/win32/mupdf.sln")" -m -nologo -v:minimal \
      -t:libmupdf -p:Configuration=Release -p:Platform=x64 -p:PlatformToolset=v143 \
      -p:ForceImportAfterCppTargets="$(cygpath -w "$src/ely.props")"
    rm -rf "$out" && mkdir -p "$out"
    cp "$src"/platform/win32/x64/Release/*.lib "$out/"
    ls "$out"
    exit 0
    ;;
esac
flags="build=release HAVE_X11=no HAVE_GLUT=no HAVE_CURL=no HAVE_LIBCRYPTO=yes tesseract=yes"
# The worker is a PIE; EL8's GCC defaults to non-PIC.
[ "$(uname)" != Linux ] || flags="$flags LIB_CFLAGS=-fPIC"
# Clang ignores Rosetta, so Intel builds name their architecture.
compilers=()
[ "$(uname)" != Darwin ] || compilers=("CC=xcrun cc -arch $(uname -m)" "CXX=xcrun c++ -arch $(uname -m)")
record="$flags${compilers[@]+ ${compilers[*]}}"
# make does not track flags; a change rebuilds all.
if [ "$(cat "$out/.flags" 2>/dev/null)" != "$record" ]; then
  rm -rf "$out"
fi
# shellcheck disable=SC2086
make -C "$src" OUT="$out" $flags ${compilers[@]+"${compilers[@]}"} -j"$jobs" libs "$out/libmupdf-pkcs7.a"
echo "$record" >"$out/.flags"
