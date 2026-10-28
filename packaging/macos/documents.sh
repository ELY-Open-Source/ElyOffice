#!/usr/bin/env bash
# Prints the app's document and URL types. Usage: documents.sh <slug>
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
slug=${1:?app slug}

# role, then the content types it opens.
case ${1:?app slug} in
  ely-doc) set -- Editor org.openxmlformats.wordprocessingml.document org.oasis-open.opendocument.text \
    com.microsoft.word.doc public.rtf ;;
  ely-table) set -- Editor org.openxmlformats.spreadsheetml.sheet org.oasis-open.opendocument.spreadsheet \
    com.microsoft.excel.xls public.comma-separated-values-text public.tab-separated-values-text ;;
  ely-show) set -- Editor org.openxmlformats.presentationml.presentation \
    org.oasis-open.opendocument.presentation com.microsoft.powerpoint.ppt ;;
  ely-pdf) set -- Editor com.adobe.pdf ;;
  ely-mark) set -- Editor net.daringfireball.markdown ;;
  ely-mail) set -- Viewer com.apple.mail.email ;;
  ely-tree) set -- Viewer public.folder ;;
  *) exit 0 ;;
esac
role=$1
shift
echo "  <key>CFBundleDocumentTypes</key><array><dict>"
echo "    <key>CFBundleTypeRole</key><string>$role</string>"
echo "    <key>LSHandlerRank</key><string>Alternate</string>"
echo "    <key>LSItemContentTypes</key><array>"
for type in "$@"; do echo "      <string>$type</string>"; done
echo "    </array>"
echo "  </dict></array>"
# URL schemes from packaging/types.tsv, as a viewer.
schemes=$(awk -F'	' -v slug="$slug" '$1 == slug && $2 ~ /:$/ { print substr($2, 1, length($2) - 1) }' "$root/packaging/types.tsv")
[ -n "$schemes" ] || exit 0
echo "  <key>CFBundleURLTypes</key><array><dict>"
echo "    <key>CFBundleURLName</key><string>$slug.links</string>"
echo "    <key>CFBundleTypeRole</key><string>Viewer</string>"
echo "    <key>CFBundleURLSchemes</key><array>"
for scheme in $schemes; do echo "      <string>$scheme</string>"; done
echo "    </array>"
echo "  </dict></array>"
