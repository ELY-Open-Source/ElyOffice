#!/usr/bin/env bash
# ElyDoc: undated notes are named; the lists stay visible.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
trap 'kill "${pid:-0}" 2>/dev/null || true; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/axv" "$root/tests/scripts/ax-values.swift"
[ "$("$work/axv" $$)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
# One dated note, twenty without a date, one tracked insertion.
python3 - "$work/notes.docx" <<'PY'
import sys, zipfile
W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
notes = ['<w:comment w:id="0" w:author="ValidAuthor" w:date="2025-03-01T10:00:00Z"><w:p><w:r><w:t>Valid note</w:t></w:r></w:p></w:comment>']
paras = []
for i in range(21):
    if i:
        notes.append(f'<w:comment w:id="{i}" w:author="Undated{i}"><w:p><w:r><w:t>Note {i}</w:t></w:r></w:p></w:comment>')
    paras.append(f'<w:p><w:commentRangeStart w:id="{i}"/><w:r><w:t>Para {i}</w:t></w:r><w:commentRangeEnd w:id="{i}"/><w:r><w:commentReference w:id="{i}"/></w:r></w:p>')
paras.append('<w:p><w:ins w:id="50" w:author="ChangeAuthor" w:date="2025-03-01T10:00:00Z"><w:r><w:t>Tracked</w:t></w:r></w:ins></w:p>')
rel = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
files = {
    '[Content_Types].xml': '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/comments.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"/></Types>',
    '_rels/.rels': f'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{rel}/officeDocument" Target="word/document.xml"/></Relationships>',
    'word/_rels/document.xml.rels': f'<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{rel}/comments" Target="comments.xml"/></Relationships>',
    'word/comments.xml': f'<?xml version="1.0" encoding="UTF-8"?><w:comments {W}>' + ''.join(notes) + '</w:comments>',
    'word/document.xml': f'<?xml version="1.0" encoding="UTF-8"?><w:document {W}><w:body>' + ''.join(paras) + '</w:body></w:document>',
}
with zipfile.ZipFile(sys.argv[1], 'w') as z:
    for name, text in files.items():
        z.writestr(name, text)
PY
ELY_LOCALE=en ELY_NO_DIALOG=1 HOME="$work" XDG_DATA_HOME="$work/data" \
  perl -e 'alarm 120; exec @ARGV' "$root/target/debug/ely-doc" "$work/notes.docx" >"$work/out" 2>"$work/err" &
pid=$!
log() { find "$work" -path '*/ElyOffice/ely-doc/ely.log' 2>/dev/null | head -n 1; }
for _ in $(seq 200); do grep -q "review lists 21 notes, 1 changes" "$(log)" 2>/dev/null && break; sleep 0.2; done
grep -q "review lists 21 notes, 1 changes" "$(log)" || fail "the lists did not read: $(tail -n 3 "$(log)")"
"$work/axv" "$pid" >/dev/null
osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid)" -e 'set frontmost to true' \
  -e 'click menu item "Comments and Changes" of menu "Review" of menu bar 1' -e 'end tell' >/dev/null
seen=
for _ in $(seq 10); do
  seen=$("$work/axv" "$pid")
  grep -q "ValidAuthor: Valid note" <<<"$seen" && break
  sleep 1
done
[ "$(grep -c "A comment by Undated[0-9]* has an unreadable time" <<<"$seen")" = 20 ] || fail "each undated note is not named"
# The valid note's center lies inside the window.
read -r top height < <(osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid)" \
  -e 'set {x, y} to position of window 1' -e 'set {w, h} to size of window 1' -e 'return (y as text) & " " & (h as text)' -e 'end tell')
at=$(grep "ValidAuthor: Valid note" <<<"$seen" | head -n 1 | sed 's/.*at=[0-9]*,//')
[ -n "$at" ] && [ "$at" -lt $((top + height)) ] || fail "the valid note sits below the window ($at, window ends $((top + height)))"
echo "ok: doc-review"
