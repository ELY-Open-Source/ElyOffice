#!/usr/bin/env bash
# The release gates: notes, the gap, the benchmark record.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() { echo "FAIL: $*" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/tasks" "$work/site/changelog" "$work/perf"
printf -- '- [~] A fixed, unreviewed\n- [x] B done\n' >"$work/tasks/TASKS.md"
printf -- '- [~] C fixed, unreviewed\n' >"$work/tasks/BUGS.md"
git -C "$work" init -q
tagged() { # tag, day
  GIT_COMMITTER_DATE="$2T12:00:00" git -C "$work" -c user.name=t -c user.email=t@t -c commit.gpgsign=false \
    commit -q --allow-empty -m "$1"
  git -C "$work" -c tag.gpgsign=false tag "$1"
}
tagged v0.1.9 2026-11-01
notes() { printf '# 0.2.0 %s\n\n## English\n\n### Known\n%s\n\n## 中文\n' "$1" "$2" >"$work/site/changelog/0.2.0.md"; }
gate() { python3 "$root/scripts/release-gate.py" 0.2.0 "$work" 2>"$work/gate.err"; }

said='- 2 fixes were made after their last review failed and have not been reviewed again.'
notes 2026-11-20 "$said"
gate || fail "true notes, nineteen days on: $(cat "$work/gate.err")"
notes 2026-11-20 '- 1 fixes were made after their last review failed and have not been reviewed again.'
! gate || fail "the notes understate the unreviewed fixes"
grep -q "2 fixes" "$work/gate.err" || fail "the count is not named: $(cat "$work/gate.err")"
notes 2026-11-04 "$said"
! gate || fail "three days after the last release with no reason"
grep -q "Released 3 days after 0.1.9" "$work/gate.err" || fail "the gap is not named: $(cat "$work/gate.err")"
notes 2026-11-04 "$said
- Released 3 days after 0.1.9: a fix for lost typing could not wait."
gate || fail "a short gap with its reason: $(cat "$work/gate.err")"
printf '# 0.2.0\n\n### Known\n%s\n' "$said" >"$work/site/changelog/0.2.0.md"
! gate || fail "notes with no date skip the gap"
grep -q "must begin: # 0.2.0 <YYYY-MM-DD>" "$work/gate.err" || fail "the missing date is not named: $(cat "$work/gate.err")"
printf '# 0.1.9 2026-11-04\n\n### Known\n%s\n' "$said" >"$work/site/changelog/0.2.0.md"
! gate || fail "notes headed with another version"
rm "$work/site/changelog/0.2.0.md"
! gate || fail "a release with no notes"

# A record: PERF-01 a start, PERF-02 a key.
record() { # day, version, start P50, key P95, reasons
  printf '# Benchmark %s\n\n```json\n{"date": "%s", "version": "%s", "machine": "Box-Linux"}\n```\n\n| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n| PERF-01 | app daily start | 30 | %s | %s ✓ | 3000 |  |  |\n| PERF-02 | app key to tile | 30 | 5 | %s ✓ | 100 |  |  |\n%s' \
    "$1" "$1" "$2" "$3" "$3" "$4" "$5" >"$work/perf/$1-Box-Linux.md"
}
base() { python3 "$root/tests/perf/baseline.py" --folder "$work/perf" --check "$@" 2>"$work/base.err"; }
record 2026-11-01 0.1.9 100 6 ''
! base 0.2.0 linux || fail "no record of this version"
record 2026-11-20 0.2.0 120 9 ''
base 0.2.0 linux || fail "a fifth slower, and three milliseconds: $(cat "$work/base.err")"
! base 0.2.0 macos || fail "a Linux record stood in for macOS"
record 2026-11-20 0.2.0 140 6 ''
! base 0.2.0 linux || fail "forty per cent slower with no reason"
grep -q "PERF-01 app daily start" "$work/base.err" || fail "the slow case is not named: $(cat "$work/base.err")"
python3 "$root/tests/perf/baseline.py" "$work/perf/2026-11-20-Box-Linux.md" >"$work/told"
grep -q -- "- PERF-01 app daily start: P50 100 → 140, P95 100 → 140 ms — $" "$work/told" || fail "the line to fill in: $(cat "$work/told")"
record 2026-11-20 0.2.0 140 6 "
$(sed -n 1p "$work/told")

- PERF-01 app daily start: P50 100 → 140, P95 100 → 140 ms — the window now waits for its fonts.
"
base 0.2.0 linux || fail "a slowdown with its reason: $(cat "$work/base.err")"
# A record without the key case, as `--only PERF-01` leaves.
sed -i '/| PERF-02 /d' "$work/perf/2026-11-20-Box-Linux.md"
! base 0.2.0 linux || fail "a record missing a case the last one had"
grep -q "PERF-02 app key to tile" "$work/base.err" || fail "the missing case is not named: $(cat "$work/base.err")"
sed -i 's/"version": "0.2.0"/"version": "0.2.0", "partial": true/' "$work/perf/2026-11-20-Box-Linux.md"
! base 0.2.0 linux || fail "a partial record stood for the release"
grep -q "no benchmark record of 0.2.0" "$work/base.err" || fail "the partial record is not refused as none: $(cat "$work/base.err")"
echo "release-gate: notes count unreviewed fixes, a short gap and a slowdown each want a reason"
