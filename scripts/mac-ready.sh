#!/usr/bin/env bash
# Before a Mac release: audio answers, notarytool reads its profile.
set -euo pipefail

profile=${1:?the notarytool keychain profile}
# A hung coreaudiod hangs the media tests, not fails them.
heard=$(python3 - <<'EOF'
import subprocess
try:
    subprocess.run(["system_profiler", "SPAudioDataType"], capture_output=True, timeout=20, check=True)
    print("yes")
except subprocess.TimeoutExpired:
    print("no answer in 20 s")
except subprocess.CalledProcessError as error:
    print(f"exit {error.returncode}: {error.stderr.decode(errors='replace').strip()[:200]}")
EOF
)
[ "$heard" = yes ] || {
  echo "mac-ready.sh: coreaudiod does not answer (system_profiler SPAudioDataType: $heard); the media tests would hang. Restart the Mac, then run again." >&2
  exit 1
}
# Its keychain must be open in this session.
if ! said=$(xcrun notarytool history --keychain-profile "$profile" --output-format json 2>&1); then
  echo "mac-ready.sh: notarytool cannot read keychain profile $profile: ${said:0:300}" >&2
  echo "mac-ready.sh: a profile that exists reads only with the login keychain unlocked in the session ci.sh runs in (log in at the Mac's console, or security unlock-keychain); a missing one is made with notarytool store-credentials" >&2
  exit 1
fi
echo "mac-ready.sh: coreaudiod answers; notarytool reads $profile"
