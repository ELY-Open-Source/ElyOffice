# PERF-10 extreme cases 2026-11-03

Format: docs/perf/README.md. Main at c3db6bd (R15.33, R15.34) merged; engine lo-snap (frozen, without patch 0080). Run E033651 is the whole list; missing-sources and engine-killed stopped there on ElyDoc's edit clock (the screen redrew 12 ms after the key, the clock line came 10 s later with the next recovery point) and were rerun as E040346.

| Case | Result | Measured | Run |
| --- | --- | --- | --- |
| ElyDoc: opens a file whose link sources are missing | FAIL | opened in 1712 ms, 5 keys max 26 ms, saved; lost on save: ['OLE link kept']; warnings: none | E040346 |
| ElyTable: opens a file whose link sources are missing | pass | opened in 2164 ms, 5 keys max 24 ms, saved; lost on save: nothing; warnings: none | E040346 |
| ElyTable: an external reference to a closed source shows the source's value | FAIL | Links!A1 shows 1.01; the cache holds 1.01, the source 2.01 | E033651 |
| ElyTable: the reference follows its source edited and saved in the same app | FAIL | Links!A1 shows 1.01 after Rates!B1 became 9.99 and was saved | E033651 |
| ElyDoc: INCLUDETEXT shows its source's text on opening | FAIL | shows the cached text | E033651 |
| ElyDoc: INCLUDETEXT follows its source changed while open | FAIL | kept the old text | E033651 |
| ElyDoc: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.docx': [3], 'b.docx': [3], 'c.docx': [3]}; command failures none | E033651 |
| ElyShow: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 0 of 9 tab switches; 18 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.pptx': [3], 'b.pptx': [3], 'c.pptx': [3]}; command failures none | E033651 |
| ElyDoc: saved 85 times while 40 keys went in | pass | keys P95 27 max 27 ms, 0 over 100; every key saved: True; save failures: none | E033651 |
| ElyTable: saved 78 times while 40 keys went in | FAIL | keys P95 65 max 77 ms, 0 over 100; every key saved: False; save failures: none | E033651 |
| ElyShow: saved 67 times while 40 keys went in | FAIL | keys P95 289 max 302 ms, 11 over 100; every key saved: True; save failures: none | E033651 |
| ElyMark: saved 139 times while 40 keys went in | pass | keys P95 16 max 17 ms, 0 over 100; every key saved: True; save failures: none | E033651 |
| ElyDoc: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E033651 |
| ElyTable: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E033651 |
| ElyShow: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E033651 |
| ElyMark: closing an autosaved document | pass | closed at once; the keys on disk True | E033651 |
| ElyDoc: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'front.docx': (10, [0]), 'other0.docx': (0, [1]), 'other1.docx': (0, [1])}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E040346 |
| ElyTable: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'front.xlsx': (10, None), 'other0.xlsx': (0, '1'), 'other1.xlsx': (0, '1')}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E040346 |
| ElyShow: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other0.pptx': (0, [1]), 'other1.pptx': (0, [1]), 'front.pptx': (15, [0])}: front keeps 15 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E040346 |
| ElyDoc: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E033651\zoom\ely-doc | E033651 |
| ElyTable: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E033651\zoom\ely-table | E033651 |
| ElyShow: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\wf_38652c95-7df-4\.work\E033651\zoom\ely-show | E033651 |
| ElyDoc: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(2913, 10840), (2537, 12583), (2403, 15726)]; memory MB [269, 348, 348, 347] | E033651 |
| ElyTable: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(3418, 12823), (3144, 11448), (2973, 11679)]; memory MB [289, 333, 339, 344] | E033651 |
| ElyShow: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(7509, 10429), (7663, 11082), (7625, 12155)]; memory MB [279, 359, 398, 427] | E033651 |
| ElyMark: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(397, 12330), (181, 10898), (368, 10234)]; memory MB [82, 83, 83, 84] | E033651 |
| ElyDoc: a 5 MB file | pass | open 2398 ms; keys P50 24 P95 32 max 32 ms; frames over 8.3 ms 42/51, dirty to shown max 21.6 ms; edit 40 ms; save 312 ms; 335 MB; problems none | E033651 |
| ElyTable: a 2 MB file | pass | open 2645 ms; keys P50 25 P95 35 max 35 ms; frames over 8.3 ms 39/56, dirty to shown max 19.9 ms; edit 26 ms; save 1112 ms; 359 MB; problems none | E033651 |
| ElyShow: a 94 MB file | FAIL | open 3151 ms; keys P50 31 P95 895 max 895 ms; frames over 8.3 ms 50/75, dirty to shown max 25.4 ms; edit 50 ms; save 985 ms; 849 MB; problems none | E033651 |
| ElyMark: a 7 MB file | pass | open 787 ms; keys P50 47 P95 77 max 77 ms; frames over 8.3 ms 153/153, dirty to shown max 62.3 ms; edit 42 ms; save 85 ms; 236 MB; problems none | E033651 |

Known and filed: ElyShow's slow keys while saving are R15.35. The ElyTable save-while-typing row is a data loss: a save ends the cell edit in progress, later keys replace the cell. The three-switching rows: after a tab switch, by UI Automation or a real mouse click, keys reach no document until the page is clicked.
