#!/usr/bin/env bash
# One chapter, one second either side, plus contact sheet.
set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
id=${1:?usage: tools/preview.sh cNN [seconds for PNG frames...]}
shift
window=$(cd "$here" && node -e "import('./scenes/index.js').then(({ default: s }) => {
  const c = s.find((x) => x.id === process.argv[1]);
  if (!c) { console.error('no scene ' + process.argv[1]); process.exit(1); }
  const end = s[s.length - 1].end;
  console.log(Math.max(0, c.start - 1), Math.min(end, c.end + 1));
})" "$id")
read -r from to <<<"$window"
out=$here/renders/preview/$id
mkdir -p "$out"
node "$here/tools/render.js" --from "$from" --to "$to" --jobs 3 --out "$out/$id.mp4" --contact "$out/$id-contact.png"
if [ "$#" -gt 0 ]; then
  node "$here/tools/frame.js" "$out" "$@"
fi
echo "preview: $out"
