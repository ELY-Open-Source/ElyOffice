# PERF-10 load 2026-11-03, 50 documents per app

Format: docs/perf/README.md.

```json
{
  "date": "2026-11-03",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Windows",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 93,
  "gpu": "GameViewer Virtual Display Adapter, AMD Radeon(TM) Graphics, NVIDIA GeForce RTX 4090",
  "system": "Microsoft Windows 11 Pro for Workstations 10.0.26300",
  "fonts": 895,
  "docs": 50,
  "rounds": 3,
  "keys": 30,
  "builds": {
    "ely-doc": "410271249d3dd1ca",
    "ely-table": "66953b1a34eb38a0",
    "ely-show": "05b496c20656cfe1",
    "ely-mark": "2ad1efd5de1deceb",
    "ely-lok-worker": "14d7a52a96dee2d1"
  },
  "lok": "C:/Users/zhang/Documents/Github/ElyOffice/engines/lo-snap/instdir/program"
}
```

Two runs: ElyTable and ElyMark from the first, ElyDoc and ElyShow from the rerun after the pointer fix.

| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 9750 | 3675 | 755 / 1093 | 35 | 30: 23 / 32 / 39 | 57/61, p95 ≤24.0, max 26.0 | 0 | 30: 19 / 33 / 9586 | 185/201, p95 ≤22.4, max 27.5 | 0 | 437 / 675 | 1681 / 2926 | 5, max 6276 |
| ElyTable | 16169 | 783 | 879 / 1210 | 45 | 30: 17 / 23 / 24 | 236/314, p95 ≤23.9, max 26.0 | 0 | 30: 23 / 36 / 5877 | 393/480, p95 ≤24.5, max 25.7 | 0 | 462 / 934 | 3500 / 6299 | 39, max 4934 |
| ElyMark | 848 | 89 | 156 / 177 | 47 | 30: 16 / 22 / 22 | 143/208, p95 ≤20.1, max 23.8 | 0 | 30: 15 / 20 / 23 | 242/321, p95 ≤20.7, max 22.0 | 0 | 812 / 1494 | 334 / 544 | 1, max 373 |

Problems:

- ElyDoc: doc-001-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-001-linked.docx: OLE link kept failed
- ElyDoc: doc-011-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-011-linked.docx: OLE link kept failed
- ElyDoc: doc-021-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-021-linked.docx: OLE link kept failed
- ElyDoc: doc-031-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-031-linked.docx: OLE link kept failed
- ElyDoc: doc-041-linked.docx: INCLUDETEXT shows the source's text failed
- ElyDoc: doc-041-linked.docx: OLE link kept failed
- ElyDoc: PERF-02: 1 of 30 keys over 100 ms editing, max 9586 ms
- ElyDoc: PERF-02: main thread busy over 100 ms 5 times, max 6276 ms
- ElyShow: run stopped: ely-show: key 1 showed nothing in 10 s
- ElyTable: table-000-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-010-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-020-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-030-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: table-040-big.xlsx: Links!A1 shows the source's value failed
- ElyTable: PERF-02: 1 of 30 keys over 100 ms editing, max 5877 ms
- ElyTable: PERF-02: main thread busy over 100 ms 39 times, max 4934 ms
- ElyMark: PERF-02: main thread busy over 100 ms 1 times, max 373 ms
