# PERF-10 load 2026-11-10, 20 documents per app

Format: docs/perf/README.md.

ElyDoc, ElyTable, ElyShow, ElyMark and ElyPDF ran at the same time, each on its own Xvfb display and in its own `load.py` (ElyPDF: `pdfs.py`, its own record). Every document, the one typed into as well, is the big sample (`--big-every 1`). The command line edits all twenty documents of an app at once, the one being typed in too, three rounds; then saves them all.

```json
{
  "date": "2026-11-10",
  "machine": "AMD-Ryzen-9-9950X3D-16-Core-Processor-Linux",
  "cpu": "AMD Ryzen 9 9950X3D 16-Core Processor",
  "cores": "32",
  "memory_gb": 91,
  "gpu": "Mesa lavapipe under Xvfb",
  "system": "Omarchy, kernel 7.2.15-1-omarchy",
  "fonts": 917,
  "docs": 20,
  "big every": 1,
  "rounds": 3,
  "keys": 30,
  "builds": {
    "ely-doc": "1bc4e18a24627321",
    "ely-lok-worker": "8cbb693a3480be98",
    "ely-table": "012915bdcb1dada4",
    "ely-show": "b2fff644d7e5513f",
    "ely-mark": "f31686591e2efa13"
  },
  "lok": "engines/"
}
```

| App | Open all ms | Open one more ms | Memory MB open / edited | Free GB, lowest | Keys idle: P50 / P95 / max ms | Frames idle: over 8.3 ms / all, dirty to shown | Stalls idle | Keys editing: P50 / P95 / max ms | Frames editing | Stalls editing | Command edit P50 / P95 ms | Save P50 / max ms | Stalls whole run |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ElyDoc | 937 | 484 | 505 / 1014 | 60 | 30: 16 / 41 / 57 | 64/64, p95 ≤36.5, max 42.7 | 0 | 30: 20 / 383 / 406 | 147/147, p95 ≤36.4, max 41.0 | 0 | 80 / 6722 | 615 / 1023 | 0, max 0 |
| ElyTable | 647 | 215 | 508 / 1048 | 58 | 30: 15 / 19 / 33 | 58/58, p95 ≤35.6, max 37.2 | 0 | 30: 16 / 159 / 172 | 79/79, p95 ≤35.4, max 40.1 | 0 | 66 / 3179 | 3788 / 6978 | 0, max 0 |
| ElyShow | 1029 | 548 | 696 / 1862 | 57 | 30: 14 / 21 / 36 | 62/62, p95 ≤37.5, max 43.0 | 0 | 30: 92 / 206 / 226 | 118/118, p95 ≤37.4, max 40.7 | 0 | 205 / 9460 | 4343 / 6543 | 0, max 0 |
| ElyMark | 650 | 105 | 563 / 1014 | 57 | 30: 21 / 30 / 45 | 210/210, p95 ≤43.2, max 45.2 | 0 | 30: 20 / 28 / 29 | 212/212, p95 ≤59.6, max 67.6 | 0 | 57 / 92 | 90 / 111 | 0, max 0 |

Problems:

- ElyDoc: PERF-02: 7 of 30 keys over 100 ms editing, max 406 ms
- ElyTable: PERF-02: 3 of 30 keys over 100 ms editing, max 172 ms
- ElyShow: PERF-02: 14 of 30 keys over 100 ms editing, max 226 ms

What the problems are: a key that arrives while the engine loads a big document for the command line, or replaces or recalculates across one, waits until that is done. No key was lost, every edit and every key was read back from the saved files, the main thread never stalled, no worker ended.
