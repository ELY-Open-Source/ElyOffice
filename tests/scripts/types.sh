#!/usr/bin/env bash
# packaging/types.tsv reaches Linux, Windows and macOS alike.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
types=$root/packaging/types.tsv
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
# Apple's UTI for each MIME type, kept apart from documents.sh.
uti() {
  case $1 in
    application/vnd.openxmlformats-officedocument.wordprocessingml.document) echo org.openxmlformats.wordprocessingml.document ;;
    application/vnd.oasis.opendocument.text) echo org.oasis-open.opendocument.text ;;
    application/msword) echo com.microsoft.word.doc ;;
    application/rtf) echo public.rtf ;;
    application/vnd.openxmlformats-officedocument.spreadsheetml.sheet) echo org.openxmlformats.spreadsheetml.sheet ;;
    application/vnd.oasis.opendocument.spreadsheet) echo org.oasis-open.opendocument.spreadsheet ;;
    application/vnd.ms-excel) echo com.microsoft.excel.xls ;;
    text/csv) echo public.comma-separated-values-text ;;
    text/tab-separated-values) echo public.tab-separated-values-text ;;
    application/vnd.openxmlformats-officedocument.presentationml.presentation) echo org.openxmlformats.presentationml.presentation ;;
    application/vnd.oasis.opendocument.presentation) echo org.oasis-open.opendocument.presentation ;;
    application/vnd.ms-powerpoint) echo com.microsoft.powerpoint.ppt ;;
    application/pdf) echo com.adobe.pdf ;;
    text/markdown) echo net.daringfireball.markdown ;;
    message/rfc822) echo com.apple.mail.email ;;
    inode/directory) echo public.folder ;;
    *) fail "no UTI known for $1" ;;
  esac
}
count=0
for slug in $(cut -f1 "$types" | sort -u); do
  desktop=$("$root/packaging/linux/package.sh" --desktop "$slug")
  windows=$("$root/packaging/windows/types.sh" "$slug")
  rows=$(awk -F'\t' -v slug="$slug" '$1 == slug' "$types")
  while IFS=$'\t' read -r _ ext mime; do
    grep -q "^MimeType=.*$mime;" <<<"$desktop" || fail "$slug: Linux misses $mime"
    if [ "${ext%:}" != "$ext" ]; then
      scheme=${ext%:}
      grep -q '^Exec=.* %U$' <<<"$desktop" || fail "$slug: Linux passes no links"
      grep -q "URLAssociations\" \"$scheme\"" <<<"$windows" || fail "$slug: Windows misses $scheme"
      grep -q 'DeleteRegValue HKCU "Software\\RegisteredApplications"' <<<"$windows" || fail "$slug: uninstall keeps $scheme"
      ! grep -q "Classes\\\\$scheme[\\\\\"]" <<<"$windows" || fail "$slug: Windows takes $scheme over"
      grep -q "<string>$scheme</string>" <<<"$("$root/packaging/macos/documents.sh" "$slug")" || fail "$slug: macOS misses $scheme"
    elif [ "$ext" = / ]; then
      grep -q 'Directory\\shell' <<<"$windows" || fail "$slug: Windows misses folders"
    else
      grep -q "\\\\.$ext\\\\OpenWithProgids" <<<"$windows" || fail "$slug: Windows misses .$ext"
      grep -q "DeleteRegValue.*\\\\.$ext\\\\OpenWithProgids" <<<"$windows" || fail "$slug: uninstall keeps .$ext"
    fi
    count=$((count + 1))
  done <<<"$rows"
  # macOS declares each MIME type's own UTI, never as owner.
  mac=$("$root/packaging/macos/documents.sh" "$slug")
  grep -q '<key>LSHandlerRank</key><string>Alternate</string>' <<<"$mac" || fail "$slug: macOS rank is not Alternate"
  declared=$(sed -n '/CFBundleURLTypes/q; s|^      <string>\(.*\)</string>$|\1|p' <<<"$mac" | sort)
  wanted=$(awk -F'\t' '$2 !~ /:$/ { print $3 }' <<<"$rows" | sort -u | while read -r mime; do uti "$mime"; done | sort)
  [ "$declared" = "$wanted" ] || fail "$slug: macOS declares $(echo $declared), the table means $(echo $wanted)"
done
# Explorer's New makes docx, xlsx, pptx and md.
fresh=0
for ext in docx xlsx pptx md; do
  slug=$(awk -F'\t' -v ext="$ext" '$2 == ext { print $1 }' "$types")
  windows=$("$root/packaging/windows/types.sh" "$slug")
  grep -qF "\\.$ext\\ShellNew\" \"FileName\" \"\$INSTDIR\\new.$ext\"" <<<"$windows" ||
    fail "$slug: New misses .$ext"
  fresh=$((fresh + 1))
done
echo "ok: types ($count file types and URL schemes on three systems, $fresh in New)"
