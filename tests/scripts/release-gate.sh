#!/usr/bin/env bash
# The release gates: notes, the gap, the benchmark record.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() { echo "FAIL: $*" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/tasks" "$work/site/changelog" "$work/docs/releases" "$work/perf"
printf -- '- [~] A fixed, unreviewed\n- [x] B done\n' >"$work/tasks/TASKS.md"
printf -- '- [~] C fixed, unreviewed\n' >"$work/tasks/BUGS.md"
git -C "$work" init -q
tagged() { # tag, day
  GIT_COMMITTER_DATE="$2T12:00:00" git -C "$work" -c user.name=t -c user.email=t@t -c commit.gpgsign=false \
    commit -q --allow-empty -m "$1"
  git -C "$work" -c tag.gpgsign=false tag "$1"
}
tagged v0.1.9 2026-11-01
# Notes: day, then extra lines in the English part.
notes() {
  printf '# 0.2.0 %s\n\n## English\n\n### Fixes\n- a fix\n%s\n\n## 中文\n\n### 修复\n- 一处修复\n' "$1" "${2:-}" >"$work/site/changelog/0.2.0.md"
}
# The release record: its lines.
told() { printf '# 0.2.0\n\n%s\n' "$1" >"$work/docs/releases/0.2.0.md"; }
gate() { python3 "$root/scripts/release-gate.py" 0.2.0 "$work" 2>"$work/gate.err"; }

said='- 2 fixes were made after their last review failed and have not been reviewed again.'
notes 2026-11-20
told "$said"
gate || fail "true notes and record, nineteen days on: $(cat "$work/gate.err")"
told '- 1 fixes were made after their last review failed and have not been reviewed again.'
! gate || fail "the record understates the unreviewed fixes"
grep -q "record must say: - 2 fixes" "$work/gate.err" || fail "the count is not named: $(cat "$work/gate.err")"
told ''
notes 2026-11-20 "$said"
! gate || fail "the count in the notes stood for the record"
notes 2026-11-04
told "$said"
! gate || fail "three days after the last release with no reason"
grep -q "record must say: - Released 3 days after 0.1.9" "$work/gate.err" || fail "the gap is not named: $(cat "$work/gate.err")"
notes 2026-11-04 '- Released 3 days after 0.1.9: a fix for lost typing could not wait.'
! gate || fail "the gap's reason in the notes stood for the record"
told "$said
- Released 3 days after 0.1.9: a fix for lost typing could not wait."
notes 2026-11-04
gate || fail "a short gap with its reason: $(cat "$work/gate.err")"
# The gap line: the notes' gap, only when short.
told "$said
- Released 2 days after 0.1.9: a fix for lost typing could not wait."
! gate || fail "a gap line with the wrong count"
grep -q "says - Released 2 days after 0.1.9; the notes' 2026-11-04 is 3 days after 0.1.9 (2026-11-01)" "$work/gate.err" || fail "the wrong count is not named with both: $(cat "$work/gate.err")"
told "$said
- Released 3 days after 0.1.8: a fix for lost typing could not wait."
! gate || fail "a gap line naming another release"
grep -q "after 0.1.8; the notes' 2026-11-04 is 3 days after 0.1.9" "$work/gate.err" || fail "the wrong release is not named: $(cat "$work/gate.err")"
told "$said
- Released 3 days after 0.1.9:"
! gate || fail "a gap line with no reason"
grep -q "gives no reason" "$work/gate.err" || fail "the missing reason is not named: $(cat "$work/gate.err")"
told "$said
- Released 3 days after 0.1.9: one.
- Released 3 days after 0.1.9: two."
! gate || fail "a gap said twice"
told "$said
- Released 19 days after 0.1.9: a reason nobody needs."
notes 2026-11-20
! gate || fail "a gap line after nineteen days"
grep -q "came 19 days before 2026-11-20; a week or more needs no reason: drop - Released 19 days after 0.1.9" "$work/gate.err" || fail "the needless line is not named: $(cat "$work/gate.err")"
told "$said
- Released 3 days after 0.1.9: a fix for lost typing could not wait."
notes 2026-11-04
# No earlier release: no gap line.
first_tag=$(git -C "$work" rev-parse v0.1.9)
git -C "$work" tag -d v0.1.9 >/dev/null
! gate || fail "a gap line with no earlier release"
grep -q "no release came before" "$work/gate.err" || fail "the missing earlier release is not named: $(cat "$work/gate.err")"
git -C "$work" -c tag.gpgsign=false tag v0.1.9 "$first_tag"
gate || fail "the short gap with its reason, retagged: $(cat "$work/gate.err")"
# The user, 2026-11-20: only New and Fixes.
for head in Engineering 'Known issues' 工程; do
  notes 2026-11-04 "
### $head
- a line"
  ! gate || fail "notes with a $head section"
  grep -q "not: $head" "$work/gate.err" || fail "the $head section is not named: $(cat "$work/gate.err")"
done
notes 2026-11-04 "
### New
- a new thing"
gate || fail "a New section: $(cat "$work/gate.err")"
mv "$work/docs/releases/0.2.0.md" "$work/record"
! gate || fail "a release with no record"
grep -q "no release record at docs/releases/0.2.0.md" "$work/gate.err" || fail "the missing record is not named: $(cat "$work/gate.err")"
sed '1s/0.2.0/0.1.9/' "$work/record" >"$work/docs/releases/0.2.0.md"
! gate || fail "a record headed with another version"
mv "$work/record" "$work/docs/releases/0.2.0.md"
printf '# 0.2.0\n\n### Fixes\n- a fix\n' >"$work/site/changelog/0.2.0.md"
! gate || fail "notes with no date skip the gap"
grep -q "must begin: # 0.2.0 <YYYY-MM-DD>" "$work/gate.err" || fail "the missing date is not named: $(cat "$work/gate.err")"
printf '# 0.1.9 2026-11-04\n\n### Fixes\n- a fix\n' >"$work/site/changelog/0.2.0.md"
! gate || fail "notes headed with another version"
rm "$work/site/changelog/0.2.0.md"
! gate || fail "a release with no notes"
notes 2026-11-20
printf -- '- [x] C reviewed\n' >"$work/tasks/BUGS.md"
! gate || fail "one fix left, the record still saying two"
grep -q "record must say: - 1 fix was made after its last review failed and has not been reviewed again" "$work/gate.err" || fail "the singular is not asked for: $(cat "$work/gate.err")"
told '- 1 fix was made after its last review failed and has not been reviewed again.'
gate || fail "one fix, said in the singular: $(cat "$work/gate.err")"

# A record (PERF-01 start, PERF-02 key) in $dir, $box.
dir=$work/perf box=Box-Linux
record() { # day, version, start P50, key P95, reasons
  printf '# Benchmark %s\n\n```json\n{"date": "%s", "version": "%s", "machine": "%s"}\n```\n\n| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n| PERF-01 | app daily start | 30 | %s | %s ✓ | 3000 |  |  |\n| PERF-02 | app key to tile | 30 | 5 | %s ✓ | 100 |  |  |\n%s' \
    "$1" "$1" "$2" "$box" "$3" "$3" "$4" "$5" >"$dir/$1-$box.md"
}
# First open record: day, version, kind, ms
first() {
  printf '# Benchmark %s\n\n```json\n{"date": "%s", "version": "%s", "machine": "%s", "kind": "%s"}\n```\n\n| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n| PERF-01 | app first open after %s | 1 | %s | %s ✓ | 3000 |  |  |\n' \
    "$1" "$1" "$2" "$box" "$3" "$3" "$4" "$4" >"$dir/$1-$box-$3.md"
}
base() { python3 "$root/tests/perf/baseline.py" --folder "$dir" --check "$@" >"$work/base.out" 2>"$work/base.err"; }
record 2026-11-01 0.1.9 100 6 ''
first 2026-11-01 0.1.9 install 1000
! base 0.2.0 linux || fail "no record of this version"
record 2026-11-20 0.2.0 120 9 ''
! base 0.2.0 linux || fail "a release with no first open after install"
grep -q "no install record of 0.2.0 on linux .* run tests/perf/firsts.py install" "$work/base.err" || fail "the missing first open is not named: $(cat "$work/base.err")"
first 2026-11-20 0.2.0 install 1100
# The user, 2026-11-20: after reboot is Windows only.
base 0.2.0 linux || fail "Linux refused for no first open after reboot: $(cat "$work/base.err")"
! base 0.2.0 macos || fail "a Linux record stood in for macOS"
box=Box-Windows
record 2026-11-01 0.1.9 100 6 ''
first 2026-11-01 0.1.9 install 1000
first 2026-11-01 0.1.9 reboot 2000
record 2026-11-20 0.2.0 120 9 ''
first 2026-11-20 0.2.0 install 1100
! base 0.2.0 windows || fail "Windows with no first open after reboot"
grep -q "no reboot record of 0.2.0 on windows .* run tests/perf/firsts.py reboot" "$work/base.err" || fail "the missing reboot is not named: $(cat "$work/base.err")"
first 2026-11-20 0.2.0 reboot 2900
! base 0.2.0 windows || fail "a first open after reboot 45% slower with no reason"
grep -q "PERF-01 app first open after reboot" "$work/base.err" || fail "the slow first open is not named: $(cat "$work/base.err")"
first 2026-11-20 0.2.0 reboot 2100
base 0.2.0 windows || fail "a fifth slower, and three milliseconds: $(cat "$work/base.err")"
box=Box-Linux
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
sed -i 's/"version": "0.2.0"/"version": "0.2.0", "void": ["PERF-02 ely-doc"]/' "$work/perf/2026-11-20-Box-Linux.md"
! base 0.2.0 linux || fail "a record with void cases"
grep -q "timed while the machine paged, redo with bench.py --redo: \['PERF-02 ely-doc'\]" "$work/base.err" || fail "the void cases are not named: $(cat "$work/base.err")"
sed -i 's/, "void": \["PERF-02 ely-doc"\]//' "$work/perf/2026-11-20-Box-Linux.md"
# A record without the key case, as `--only PERF-01` leaves.
sed -i '/| PERF-02 /d' "$work/perf/2026-11-20-Box-Linux.md"
! base 0.2.0 linux || fail "a record missing a case the last one had"
grep -q "PERF-02 app key to tile" "$work/base.err" || fail "the missing case is not named: $(cat "$work/base.err")"
sed -i 's/"version": "0.2.0"/"version": "0.2.0", "partial": true/' "$work/perf/2026-11-20-Box-Linux.md"
! base 0.2.0 linux || fail "a partial record stood for the release"
grep -q "no benchmark record of 0.2.0" "$work/base.err" || fail "the partial record is not refused as none: $(cat "$work/base.err")"

# The user, 2026-11-20: Linux 0.1.17 has no install record.
dir=$work/noinstall box=Box-Linux
mkdir -p "$dir"
record 2026-11-19 0.1.17 100 6 ''
base 0.1.17 linux || fail "Linux 0.1.17 refused for no install record: $(cat "$work/base.err")"
grep -q "linux 0.1.17 ships with no install record" "$work/base.out" || fail "the exception is not said: $(cat "$work/base.out")"
record 2026-11-20 0.2.0 100 6 ''
! base 0.2.0 linux || fail "Linux 0.2.0 passed with no install record"

# The user, 2026-11-20: 0.1.17 keeps 0.1.16's bench record, no other.
for system in macos linux; do
  dir=$work/earlier-$system box=Box-Mac other=Box-Linux
  [ "$system" = macos ] || { box=Box-Linux other=Box-Mac; }
  mkdir -p "$dir"
  # Linux 0.1.17 has no install record either.
  [ "$system" = linux ] || first 2026-11-19 0.1.17 install 1000
  record 2026-11-10 0.1.15 100 6 ''
  record 2026-11-21 0.1.18 100 6 ''
  box=$other record 2026-11-12 0.1.16 100 6 ''
  ! base 0.1.17 "$system" || fail "$system: an older, later or other-platform record stood in for 0.1.16's"
  grep -q "no benchmark record of 0.1.17 on $system" "$work/base.err" || fail "$system: the refusal is not named: $(cat "$work/base.err")"
  record 2026-11-12 0.1.16 100 6 ''
  base 0.1.17 "$system" || fail "$system 0.1.17 refused with 0.1.16's record: $(cat "$work/base.err")"
  grep -q "$system 0.1.17 has no bench record of its own and uses 2026-11-12-$box.md (0.1.16)" "$work/base.out" || fail "$system: the stand-in is not said: $(cat "$work/base.out")"
  ! base 0.1.19 "$system" || fail "$system: a version outside the decision took an earlier record"
done
echo "release-gate: notes hold only New and Fixes; the record counts unreviewed fixes; a short gap and a slowdown each want a reason, the gap line matching the notes' day and absent after a week; first opens after install, and after reboot on Windows, are required; a record timed while paging is refused; the user's 0.1.17 exceptions hold and go no further"
