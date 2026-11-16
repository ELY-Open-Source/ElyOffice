# Benchmark 2026-11-16

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-16",
  "machine": "13th-Gen-Intel-R-Core-TM-i5-13500H-Windows",
  "cpu": "13th Gen Intel(R) Core(TM) i5-13500H",
  "cores": "8",
  "memory_gb": 23,
  "gpu": "Red Hat VirtIO GPU DOD controller",
  "system": "Microsoft Windows 11 Pro 10.0.26200",
  "fonts": 535,
  "version": "0.1.16",
  "kind": "reboot"
}
```

| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PERF-01 | ElyDoc doc.docx first open after reboot | 1 | 3109 | 3109 ✗ | 3000 |  |  |
| PERF-01 | ElyTable sheet.xlsx first open after reboot | 1 | 1028 | 1028 ✓ | 3000 |  |  |
| PERF-01 | ElyShow deck.pptx first open after reboot | 1 | 2238 | 2238 ✓ | 3000 |  |  |

PERF-09 (Agent sessions) belongs to the second release.

Microsoft Office, to a window titled with the file; recorded, not gated:

- WINWORD.EXE doc.docx: 4047 ms
- EXCEL.EXE sheet.xlsx: 2406 ms
- POWERPNT.EXE deck.pptx: 1859 ms
