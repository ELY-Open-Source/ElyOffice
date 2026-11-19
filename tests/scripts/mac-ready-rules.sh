#!/usr/bin/env bash
# mac-ready.sh against stand-ins: hung audio, an unreadable profile.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
command -v python3 >/dev/null || { echo "SKIP: no python3"; exit 0; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/bin"
# Stand-ins: audio answers or hangs; notarytool reads or refuses.
audio() { printf '#!/usr/bin/env bash\n%s\n' "$1" >"$work/bin/system_profiler"; chmod +x "$work/bin/system_profiler"; }
notary() { printf '#!/usr/bin/env bash\n%s\n' "$1" >"$work/bin/xcrun"; chmod +x "$work/bin/xcrun"; }
ready() { PATH=$work/bin:$PATH bash "$root/scripts/mac-ready.sh" ely-notary 2>&1; }

audio 'echo "Audio:"'
notary 'echo "{\"history\": []}"'
out=$(ready) || fail "a sound Mac is refused: $out"
[[ $out == *"coreaudiod answers; notarytool reads ely-notary"* ]] || fail "no all-clear: $out"

notary 'echo "Error: No Keychain password item found for profile: ely-notary" >&2; exit 69'
if out=$(ready); then fail "an unreadable profile passed: $out"; fi
[[ $out == *"cannot read keychain profile ely-notary: Error: No Keychain password item found"* ]] || fail "notarytool's words are lost: $out"
[[ $out == *"login keychain unlocked"*"notarytool store-credentials"* ]] || fail "no way out named: $out"

notary 'echo "{\"history\": []}"'
audio 'exec sleep 60'
started=$SECONDS
if out=$(ready); then fail "hung audio passed: $out"; fi
[[ $out == *"coreaudiod does not answer (system_profiler SPAudioDataType: no answer in 20 s)"* ]] || fail "the hang is not named: $out"
[ $((SECONDS - started)) -lt 40 ] || fail "the check waited $((SECONDS - started)) s"

audio 'echo "no audio service" >&2; exit 1'
if out=$(ready); then fail "failed audio passed: $out"; fi
[[ $out == *"system_profiler SPAudioDataType: exit 1: no audio service"* ]] || fail "the failure is not named: $out"
echo "PASS: mac-ready.sh passes a sound Mac and names hung audio and an unreadable profile"
