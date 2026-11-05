PASS

Round-1 defects, all fixed:

- R3.4 (tasks/TASKS.md:248) stays `[ ]` and ends “未送审”. The facts still hold: ee1b8ad, tag v0.1.4, 46 assets.
- R13.7 (tasks/TASKS.md:402) stays `[ ]`. It says the Nix step for 0.1.12 was not done: release.json went from 0.1.11 straight to 0.1.13 in 4ba153b (R14.54, `[x]`). v0.1.12 has 89 assets, published 2026-10-26.
- C12.23 (tasks/TASKS.md:159) stays `[ ]` with “未送审”, and it names the 120 Hz screen and the Raspberry Pi as untested.
- R12.7 (tasks/TASKS.md:360) now says eight x86_64 dmgs, seven apps plus ElyOffice. v0.1.11, v0.1.12 and v0.1.13 each have 8. 7d4bcf4 is first in v0.1.11.

No new defects in the changed lines:

- R2.3: r1 to r3 are FAIL, and 326fe24 changes `site/video/scenes/verify.mjs`.
- L1.6: `linux-mark-idle.sh` is gone, and `outside.rs` and `linux-idle.sh` exist.
- R15.43: 5f5dbdf and the cited extreme report exist, and R15.55 is `[x]` with an r1 PASS.
- R7.7: 41321fd exists and R14.43-r2 is PASS.
- R15.11c: R15.11h-r3 and R15.11k-r2 are PASS.
- C22.1 and C24.2: R14.47-r3 and R14.49-r3 are PASS.
- C08.6 and C19.1: R15.11 is `[~]`.

Only status markers changed and text was added; nothing existing was rewritten. `tests/scripts/records.py`, `scripts/check.sh` and `scripts/test-map.py` pass, and test-map.py leaves no diff.
