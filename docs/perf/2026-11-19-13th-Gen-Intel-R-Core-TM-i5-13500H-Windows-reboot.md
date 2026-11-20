# Benchmark 2026-11-19

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-19",
  "machine": "13th-Gen-Intel-R-Core-TM-i5-13500H-Windows",
  "cpu": "13th Gen Intel(R) Core(TM) i5-13500H",
  "cores": "8",
  "memory_gb": 23,
  "gpu": "Red Hat VirtIO GPU DOD controller",
  "system": "Microsoft Windows 11 Pro 10.0.26200",
  "fonts": 535,
  "version": "0.1.17",
  "kind": "reboot"
}
```

| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Total memory MB | Budget MB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PERF-01 | ElyDoc doc.docx first open after reboot | 1 | 2216 | 2216 ✓ | 3000 |  |  |
| PERF-01 | ElyTable sheet.xlsx first open after reboot | 1 | 1200 | 1200 ✓ | 3000 |  |  |
| PERF-01 | ElyShow deck.pptx first open after reboot | 1 | 2009 | 2009 ✓ | 3000 |  |  |

PERF-09 (Agent sessions) belongs to the second release.

Microsoft Office, to a window titled with the file; recorded, not gated:

- WINWORD.EXE doc.docx: 5078 ms
- EXCEL.EXE sheet.xlsx: 2453 ms
- POWERPNT.EXE deck.pptx: 1563 ms
