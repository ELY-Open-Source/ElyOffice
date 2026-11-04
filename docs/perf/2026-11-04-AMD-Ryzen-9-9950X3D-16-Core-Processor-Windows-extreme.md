# PERF-10 extreme cases 2026-11-04

Format: docs/perf/README.md. Branch r15-31 from main 3e2fbef (R15.33 to R15.54); engine lo-src with patches 0001 to 0088 (`ELY_LOK_DIR` engines/lo-src/instdir/program). Full run E170610, no case voided. Two cases ran again after tool fixes: E172819 (changed sources split in two, so ElyDoc's half runs when ElyTable's fails; three-switching counts typed x case-blind, as Impress autocorrect capitalizes a run's first x: E170610 read 7 of 9 keys per deck, all 9 were saved), E173626 (a refused source save is retried and timed, so the rest of the case runs).

| Case | Result | Measured | Run |
| --- | --- | --- | --- |
| ElyDoc: opens a file whose link sources are missing | pass | opened in 2179 ms, 5 keys max 26 ms, saved; lost on save: nothing; warnings: none | E170610 |
| ElyTable: opens a file whose link sources are missing | pass | opened in 1850 ms, 5 keys max 18 ms, saved; lost on save: nothing; warnings: none | E170610 |
| ElyTable: the source saves right after Update links read it | FAIL | refused for 61 s: rktrees\r15-31\.work\E173626\changed\ely-table\source.xlsx: Access is denied. (os error 5) | E173626 |
| ElyTable: an external reference shows its saved value on opening | pass | Links!A1 shows 1.01, wants 1.01 | E173626 |
| ElyTable: Update links reads the source | pass | Links!A1 shows 2.01, wants 2.01 | E173626 |
| ElyTable: a later save of the source waits for the user | pass | Links!A1 shows 2.01, wants 2.01 | E173626 |
| ElyTable: Update links again reads the saved source | pass | Links!A1 shows 9.99, wants 9.99 | E173626 |
| ElyDoc: INCLUDETEXT shows its saved text on opening | pass | read with text get | E172819 |
| ElyDoc: Update links reads INCLUDETEXT's source | pass | read with text get | E172819 |
| ElyDoc: a later save of the source waits for the user | pass | read with text get | E172819 |
| ElyDoc: Update links again reads the saved source | pass | read with text get | E172819 |
| ElyDoc: INCLUDETEXT saved with the changed text | pass | read in the saved file after the sources changed; screens docs/perf/2026-11-04-links/ | E172819 |
| ElyDoc: the linked picture keeps its link | pass | read in the saved file after the sources changed; screens docs/perf/2026-11-04-links/ | E172819 |
| ElyDoc: the OLE link stays | pass | read in the saved file after the sources changed; screens docs/perf/2026-11-04-links/ | E172819 |
| ElyDoc: three open, tabs switched while typing and while all are edited | pass | keys stay in the document after 9 of 9 tab switches; 27 keys, 0 over 100 ms; keys lost (saved, typed): none; edits saved {'a.docx': [3], 'b.docx': [3], 'c.docx': [3]}; command failures none | E172819 |
| ElyShow: three open, tabs switched while typing and while all are edited | FAIL | keys stay in the document after 9 of 9 tab switches; 27 keys, 6 over 100 ms; keys lost (saved, typed): none; edits saved {'a.pptx': [3], 'b.pptx': [3], 'c.pptx': [3]}; command failures none | E172819 |
| ElyDoc: saved 43 times while 40 keys went in | pass | keys P95 26 max 26 ms, 0 over 100; every key saved: True; save failures: none | E170610 |
| ElyTable: saved 42 times while 40 keys went in | pass | keys P95 23 max 27 ms, 0 over 100; every key saved: True; save failures: none | E170610 |
| ElyShow: saved 42 times while 40 keys went in | pass | keys P95 29 max 31 ms, 0 over 100; every key saved: True; save failures: none | E170610 |
| ElyMark: saved 157 times while 40 keys went in | pass | keys P95 17 max 17 ms, 0 over 100; every key saved: True; save failures: none | E170610 |
| ElyDoc: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E170610 |
| ElyTable: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E170610 |
| ElyShow: closing an edited document asks, cancels and saves | pass | asked True; Escape kept it with its keys True; Save closed it True; the keys on disk True | E170610 |
| ElyMark: closing an autosaved document | pass | closed at once; the keys on disk True | E170610 |
| ElyDoc: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'front.docx': (10, [0]), 'other1.docx': (0, [1]), 'other0.docx': (0, [1])}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E170610 |
| ElyTable: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other0.xlsx': (0, '1'), 'other1.xlsx': (0, '1'), 'front.xlsx': (10, None)}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E170610 |
| ElyShow: the engine killed mid-edit | pass | workers killed 2; app alive True; noticed True; reopened as ['Untitled 1', 'Untitled 2', 'Untitled 3']; the points hold (keys, edit) {'other1.pptx': (0, [1]), 'other0.pptx': (0, [1]), 'front.pptx': (10, [0])}: front keeps 10 of 15 keys, 10 typed before the last point; the 5 after it are lost by the 10 s point interval | E170610 |
| ElyDoc: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\r15-31\.work\E170610\zoom\ely-doc | E170610 |
| ElyTable: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\r15-31\.work\E170610\zoom\ely-table | E170610 |
| ElyShow: zoom 25 % to 400 % and by wheel | pass | tiles failed: 0; zoom not as asked: none; screenshots in C:\Users\zhang\Documents\Github\ElyOffice\.claude\worktrees\r15-31\.work\E170610\zoom\ely-show | E170610 |
| ElyDoc: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(2547, 8189), (2426, 8195), (2451, 8177)]; memory MB [269, 345, 349, 338] | E170610 |
| ElyTable: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(2539, 8054), (2565, 8077), (2631, 8044)]; memory MB [293, 315, 347, 339] | E170610 |
| ElyShow: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(6881, 8171), (6848, 8111), (6839, 8078)]; memory MB [271, 352, 370, 403] | E170610 |
| ElyMark: 20 handed over at once and closed by key, 3 times | pass | open / close ms [(325, 8107), (141, 8081), (153, 8119)]; memory MB [83, 111, 111, 87] | E170610 |
| ElyDoc: a 5 MB file | pass | open 2001 ms; keys P50 20 P95 30 max 30 ms; frames over 8.3 ms 25/29, dirty to shown max 21.0 ms; edit 29 ms; save 261 ms; 330 MB; problems none | E170610 |
| ElyTable: a 2 MB file | pass | open 2137 ms; keys P50 16 P95 21 max 21 ms; frames over 8.3 ms 31/37, dirty to shown max 18.6 ms; edit 18 ms; save 1563 ms; 360 MB; problems none | E170610 |
| ElyShow: a 94 MB file | pass | open 3109 ms; keys P50 32 P95 35 max 35 ms; frames over 8.3 ms 39/39, dirty to shown max 24.0 ms; edit 41 ms; save 1394 ms; 842 MB; problems none | E170610 |
| ElyMark: a 7 MB file | pass | open 1042 ms; keys P50 44 P95 77 max 77 ms; frames over 8.3 ms 167/167, dirty to shown max 55.4 ms; edit 24 ms; save 84 ms; 239 MB; problems none | E170610 |

The two failures. ElyTable: after Update links reads source.xlsx, the kernel holds the file for about a minute; a save over it (Ely's own atomic replace, or any program's rename) is refused with Access is denied. Outside the case, `os.replace` onto the source failed 0, 15, 30 and 45 s after Update links and worked from 60 s on (`.work/R15.31/lockrepro.py`, local). ElyShow: every slow key is the first key after a 2.5 s rest; it waited 71 to 154 ms for a recovery point that began 220 to 260 ms before its tile (the residual R15.35 recorded).

Screens of the linked picture and the OLE preview, cropped to the banner and page (docs/perf/2026-11-04-links/): 1, opened: banner, INCLUDETEXT and the OLE preview show what the file saved; the picture shows the current picture.png, since a DOCX keeps no copy of a linked picture. 2, Update links: the preview draws the source's rates (1 2.01 to 5 2.05), INCLUDETEXT reads "Included after". 3, sources changed on disk: the banner names include.docx and offers Update links; the page does not change. 4, Update links again: preview 9.01 to 9.05, "Included changed", the picture is the new file. Which picture shows was checked by pixels: each noisy region, cut from the OLE preview where their widths part and scaled to 64 px, correlates with the file it should show 0.38 (opened, changed), 0.49 (updated) and 0.50 (renewed), and under 0.01 with the other (`.work/R15.31/compare.py`, local).
