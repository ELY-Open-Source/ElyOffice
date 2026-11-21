# Benchmark 2026-11-20

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-20",
  "machine": "13th-Gen-Intel-R-Core-TM-i5-13500H-Windows",
  "cpu": "13th Gen Intel(R) Core(TM) i5-13500H",
  "cores": "8",
  "memory_gb": 23,
  "gpu": "Red Hat VirtIO GPU DOD controller",
  "system": "Microsoft Windows 11 Pro 10.0.26200",
  "fonts": 535,
  "version": "0.1.17",
  "kind": "install"
}
```

| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Resident MB | Budget MB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PERF-01 | ElyDoc doc.docx first open after install | 1 | 2945 | 2945 ✓ | 3000 |  |  |
| PERF-01 | ElyTable sheet.xlsx first open after install | 1 | 605 | 605 ✓ | 3000 |  |  |
| PERF-01 | ElyShow deck.pptx first open after install | 1 | 2611 | 2611 ✓ | 3000 |  |  |

PERF-09 (Agent sessions) belongs to the second release.

Microsoft Office, to a window titled with the file; recorded, not gated:

- WINWORD.EXE doc.docx: 4234 ms
- EXCEL.EXE sheet.xlsx: 1265 ms
- POWERPNT.EXE deck.pptx: 1593 ms
Slower than the last record by over 30%:

- PERF-01 ElyDoc doc.docx first open after install: P50 1724 → 2945, P95 1724 → 2945 ms — Defender's cloud lookups just after the install, as R20.62 found: MPLog has 40 cloud requests in the minute 04:29 UTC and none from 04:23 to 04:26; the idle engine was taken in 0 ms, and the kernel's load of doc.docx took 1653 ms of the 2945, across two of them (04:29:29.565 to 30.201, 31.262 to 31.682). MPLog does not say which file was looked up; R20.62 is the item that finds it. 2945 ms is 55 ms under the 3000 ms target. The 0.1.17 setup this record supersedes took 3158 ms.
