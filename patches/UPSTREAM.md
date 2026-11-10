# Patches that belong upstream

These fix defects in the upstream code itself. They need nothing from Ely and no LibreOfficeKit client to matter. Once upstream takes one, the next pin drops it from the series. The other patches add commands and behavior a kit client needs; they stay here.

None has been sent yet (2026-11-10). Sending needs the owner's accounts: LibreOffice takes changes through Gerrit with the contributor's license statement, MuPDF through Artifex's bug tracker with its contributor agreement, the crates through pull requests on GitHub. Before sending, drop the Ely task and issue numbers from each message; the rest stands as written.

## LibreOffice (core, Gerrit)

| Patch | Module | Defect | Seen without Ely |
| --- | --- | --- | --- |
| 0032 | oox chart export | A scatter chart whose X values come from numeric categories is written with a string reference; after a round trip X reads 1, 2, 3. | Save such a chart to PPTX in Impress, reopen. |
| 0039 | vcl, macOS Skia | The first text draw builds a font manager from every system font and never reads it: about 175 ms on the main thread with 370 fonts. | Type the first character into a new document on macOS. |
| 0060 | external/liblangtag | The Windows `LT_LOCK` makes a new mutex on each call, never waits on it, never closes it: no locking, one leaked handle per lock. | Watch the handle count over repeated loads on Windows. Also for liblangtag's own tracker. |
| 0063 | package | A zip entry name with a backslash (`word\theme/`, as Tencent Docs writes) makes the package "broken". | Open such a DOCX. |
| 0065 | oox theme export | The font scheme drops every `<a:font script= typeface=>`; East Asian theme fonts resolve to nothing on reopen and pages reflow. | Round-trip a Word DOCX whose theme maps Hans. |
| 0066 | sw DOCX export | The footnote separator paragraph is written bare; Word gives it Normal's spacing, about 10 pt a page. | Export footnotes to DOCX, open in Word. |
| 0068 | starmath OOXML import | A hyphen in a math run is escaped as quoted text; Word shows U+2212. Export writes it back as normal text. | Round-trip `a-b` in an OMML equation. |
| 0070 | sw writerfilter | The PAGEREF refresh during load marks the SwDoc modified while the shell refuses the mark; the untouched file later asks to be saved. | Open a DOCX with PAGEREF fields, close it. |
| 0073 | formula compiler, OOXML | A bare `TRUE` or `FALSE` in a formula is written back as `TRUE()`; every save changes the text Excel shows. | Round-trip `=IF(A1,TRUE,FALSE)` through XLSX. |
| 0074 | starmath OOXML import | Double-struck runs keep only C, N, Q, R, Z; other letters and digits come back plain, and lowercase c, n, q, r, z become unknown names. | Round-trip a double-struck 3. |
| 0075 | sw undo | `impl_DoUndoRedo` takes the current view's shell even when that view shows another document; an undo during a second document's import moves the first one's cursor past its nodes. | Under LOK, load a DOCX with two paragraphs of undefined style while another document is open. |
| 0077 | sc XLSX import | A hyperlink location that already starts with `#` gets a second one; the jump never resolves. | Open an XLSX whose location is `#'Sheet'!A1`. |
| 0080 | sal, Windows | A path of exactly 248 characters gets no `\\?\` prefix; `CreateDirectoryW` fails with error 206. | A profile path of 149 to 162 characters; the extension manager's folders land on 248. |
| 0104 | external/liborcus | With the internal zlib, liborcus links no zlib; the first load ends with `undefined symbol: inflateInit2_`. | A Linux build with `--without-system-zlib`; open a damaged spreadsheet. |

Larger changes to the filters fix real losses too, but each changes what Calc or Writer keeps, and upstream will want to weigh them: 0049 (inline objects follow `w:position`), 0059 (results of unknown functions stay), 0067 (dynamic arrays), 0071, 0079, 0083, 0085 (pivot captions and extents), 0072 (video click sequence), 0078 and 0084 (where a TOC field ends). 0061 and 0093 only add includes that 0059 and 0067 need; they go with those.

## MuPDF (bugs.ghostscript.com)

| Patch | Defect |
| --- | --- |
| 0001 | `pdf_save_snapshot` marks the incremental sections saved; undo after a snapshot restores nothing. |
| 0002 | `pdf_add_embedded_file` never ends its operation on success; `pdf_set_link_uri` leaves an old `/Dest` that readers take first. |
| 0004 | A signature's reason and location go into the appearance only, not the signature dictionary. |
| 0005 | A save that cannot open its file leaves `save_in_progress` set; later edits bypass the journal. |
| 0006 | On Windows `fz_fopen_utf8` never shares delete; no rename can replace an open document. |
| 0007 | The sanitize filter sends no stroke state before text; stroked text loses its width and dash. |
| 0008 | The sanitize filter drops marked content that was empty in the source (`/Artifact BMC EMC`). |

0003 (OCR fails when a requested language does not load) changes a choice Tesseract's caller makes; it is a proposal, not a defect report.

## Crates (GitHub)

| Crate | Commit here | Defect |
| --- | --- | --- |
| zbus 5.19.0 | `9fa71c98` | Removing a path's last interface leaves its empty ancestors on the bus. |
| mail-parser 0.11.9 | `33f298ee` | A quoted local part before `@` falls out of the address. |
| accesskit_atspi_common 0.19.1 | `ca4ee86b` | A dropped adapter leaves its nodes on the bus. |
| accesskit_unix 0.22.1 | `3cf3652b` | Cache updates go out field by field; AT-SPI clients expect one struct, and a node that gains an interface is not announced again. |

`accesskit_atspi_common` also answers sentence queries here (`85f6faca` and after): a feature for AccessKit to judge, with a splitter that knows eight languages' punctuation, not Unicode's rules. The gpui changes are not sorted here yet; they wait for their patch folder (task R18.23).
