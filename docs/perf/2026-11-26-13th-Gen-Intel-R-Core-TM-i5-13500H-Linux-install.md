# Benchmark 2026-11-26

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-26",
  "machine": "13th-Gen-Intel-R-Core-TM-i5-13500H-Linux",
  "cpu": "13th Gen Intel(R) Core(TM) i5-13500H",
  "cores": "8",
  "memory_gb": 7,
  "gpu": "llvmpipe (LLVM 23.1.3, 256 bits)",
  "system": "Omarchy, kernel 7.2.15-1-omarchy",
  "fonts": 766,
  "version": "0.1.18",
  "kind": "install"
}
```

| PERF | Case | Runs | P50 ms | P95 ms | Target ms | Resident MB | Budget MB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PERF-01 | ElyDoc doc.docx first open after install | 1 | 1192 | 1192 ✓ | 3000 |  |  |
| PERF-01 | ElyTable sheet.xlsx first open after install | 1 | 374 | 374 ✓ | 3000 |  |  |
| PERF-01 | ElyShow deck.pptx first open after install | 1 | 1040 | 1040 ✓ | 3000 |  |  |

PERF-09 (Agent sessions) belongs to the second release.
