# PERF-10 extreme cases 2026-11-03

Format: docs/perf/README.md. Newest result of each case; run is the work folder.

| Case | Result | Measured | Run |
| --- | --- | --- | --- |
| ElyDoc: opens a file whose link sources are missing | FAIL | opened in 2621 ms, 5 keys max 26 ms, saved; lost on save: ['OLE link kept']; warnings: none | E000625 |
| ElyTable: opens a file whose link sources are missing | pass | opened in 1824 ms, 5 keys max 23 ms, saved; lost on save: nothing; warnings: none | E000625 |
| ElyTable: an external reference to a closed source shows the source's value | FAIL | Links!A1 shows 1.01; the cache holds 1.01, the source 2.01 | E000625 |
| ElyTable: the reference follows its source edited and saved in the same app | FAIL | Links!A1 shows 1.01 after Rates!B1 became 9.99 and was saved | E000625 |
| ElyDoc: INCLUDETEXT shows its source's text on opening | FAIL | shows the cached text | E000625 |
| ElyDoc: INCLUDETEXT follows its source changed while open | FAIL | kept the old text | E000625 |
| ElyDoc: saved 61 times while 40 keys went in | pass | keys P95 38 max 67 ms; every key saved: True; save failures: none | E000625 |
| ElyTable: saved 57 times while 40 keys went in | FAIL | keys P95 78 max 375 ms; every key saved: False; save failures: none | E000625 |
| ElyShow: saved 50 times while 40 keys went in | pass | keys P95 326 max 353 ms; every key saved: True; save failures: none | E000625 |
| ElyMark: saved 144 times while 40 keys went in | pass | keys P95 17 max 18 ms; every key saved: True; save failures: none | E000625 |
| ElyDoc: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other0.docx': (0, [1]), 'front.docx': (10, [0]), 'other1.docx': (0, [1])}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E000625 |
| ElyTable: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other1.xlsx': (0, '1'), 'other0.xlsx': (0, '1'), 'front.xlsx': (10, None)}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E000625 |
| ElyShow: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other0.pptx': (0, [1]), 'front.pptx': (10, [0]), 'other1.pptx': (0, [1])}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E000625 |
| ElyDoc: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; screenshots for review in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E004209\zoom\ely-doc | E004209 |
| ElyTable: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; screenshots for review in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E004209\zoom\ely-table | E004209 |
| ElyShow: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; screenshots for review in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E004209\zoom\ely-show | E004209 |
| ElyDoc: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(10758, 17600), (3237, 13556), (4322, 15455)]; memory MB [269, 343, 348, 352] | E000625 |
| ElyTable: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(4185, 14031), (4959, 11847), (3328, 11835)]; memory MB [284, 316, 341, 350] | E000625 |
| ElyShow: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(9106, 23125), (11526, 21522), (10900, 12733)]; memory MB [282, 376, 410, 447] | E000625 |
| ElyMark: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(965, 12162), (219, 14713), (238, 13756)]; memory MB [80, 83, 83, 83] | E000625 |
| ElyDoc: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.docx': [3], 'b.docx': [3], 'c.docx': [3]}; command failures none | E002800 |
| ElyShow: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.pptx': [3], 'b.pptx': [3], 'c.pptx': [3]}; command failures none | E002800 |
| ElyDoc: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E002800 |
| ElyTable: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E002800 |
| ElyShow: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E002800 |
| ElyMark: closing an autosaved document | pass | closed at once; the keys on disk True | E002800 |
| ElyDoc: a huge file | FAIL | stopped: ely-doc: key 17 showed nothing in 10 s | E004209 |
| ElyTable: a 2 MB file | pass | open 4659 ms; keys P50 34 P95 69 max 69 ms; edit 171 ms; save 1237 ms; 367 MB; problems none | E004209 |
| ElyShow: a 94 MB file | FAIL | open 4894 ms; keys P50 37 P95 1115 max 1115 ms; edit 169 ms; save 1249 ms; 849 MB; problems none | E004209 |
| ElyMark: a 7 MB file | FAIL | open 327 ms; keys P50 123 P95 237 max 237 ms; edit 32 ms; save 90 ms; 229 MB; problems none | E004209 |

Notes: the ElyTable zoom row passes on tiles only; every zoom step scrolls the view away from A1 (Ctrl+Home does not bring it back), so the selection border against the cell at 67 %, 125 % and 400 % was not seen. ElyDoc's huge-file stop is the edit clock missing a key the engine took and redrew (logged later at the next recovery point), not a hang. The three-switching rows: after a tab switch, by UI Automation or a real mouse click, keys reach no document until the page is clicked.
