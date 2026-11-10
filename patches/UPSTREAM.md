# Patches that belong upstream

These fix defects in the upstream code itself. None needs Ely: the first table's defects show in LibreOffice proper, the second's in LibreOfficeKit as upstream ships it, for any client. Once upstream takes one, the next pin drops it from the series. The other patches add commands, values and choices Ely's client needs; they stay here.

None has been sent yet (2026-11-10). Sending needs the owner's accounts: LibreOffice takes changes through Gerrit with the contributor's license statement, MuPDF through Artifex's bug tracker with its contributor agreement, the crates through pull requests on GitHub. Before sending, drop the Ely task and issue numbers from each message; the rest stands as written.

## LibreOffice (core, Gerrit)

| Patch | Module | Defect | Seen without Ely |
| --- | --- | --- | --- |
| 0015 | vcl, macOS | `AquaSalInstance` enters the native wait though a timer is already due (`IsDirectTimeout`); with no native event coming, the timer runs only when something else wakes the loop. | Any idle job scheduled with a zero timeout on macOS while no input arrives. |
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
| 0077 | sc XLSX import | A hyperlink location that already starts with `#` gets a second one; the jump never resolves. | Open an XLSX whose location is `#'Sheet'!A1`. |
| 0080 | sal, Windows | A path of exactly 248 characters gets no `\\?\` prefix; `CreateDirectoryW` fails with error 206. | A profile path of 149 to 162 characters; the extension manager's folders land on 248. |
| 0104 | external/liborcus | With the internal zlib, liborcus links no zlib; the first load ends with `undefined symbol: inflateInit2_`. | A Linux build with `--without-system-zlib`; open a damaged spreadsheet. |

## LibreOfficeKit (core, Gerrit)

Defects of the kit's existing calls and callbacks. No new command or value comes with them.

| Patch | Where | Defect | Seen with any client |
| --- | --- | --- | --- |
| 0002 | desktop `doc_postUnoCommand` | The command is dispatched with no frame, so it goes to the desktop's active frame: a command posted to document A runs in document B. | Load two documents, post a command to the one loaded first. |
| 0005 | vcl jsdialog | A send from `JSDialogNotifyIdle` can make the client close the dialog; the idle then goes on with its destroyed self. A disposed dialog is also still described. | Close a jsdialog from inside the callback that announces it. |
| 0011 | desktop, fonts-missing | A document font given as a `;` list is reported missing unless the whole list equals one used name. | Open a file whose font is `A;B` with B installed. |
| 0024 | sfx2 `LokStarMathHelper` | `PaintAllInPlaceOnTile` resizes the tile device with the default erase: every Impress tile painted during text edit comes back blank. | Paint a tile while editing text on a slide. |
| 0025 | sd `SdXImpressDocument::GetViewShell` | It asks the doc shell for its connected view, which Impress disconnects while the shell stack reshuffles; a request in that gap paints a blank tile and drops keys. | Post keys and paint right after a pane change. |
| 0048 | desktop, fonts-missing | Names compare as written, not as the font lookup matches them: `宋体` found as SimSun is reported missing. | Open a file naming a font by its localized or differently cased name. |
| 0052 | sd `getPartInfo` | In the master view `getParts` and `getPartName` speak of masters, `getPartInfo` of the slide at the same index; past the slide count the info is empty. | Switch to the master view of a deck with more masters than slides. |
| 0064 | desktop `LOKInteractionHandler` | A broken package's repair question falls to the standard handler, a modal dialog nobody can answer: the load never returns. | Load a damaged ODF or OOXML package. |
| 0069 | svx media object | Painting a slide with a video builds a hidden `MediaWindow`; its player outlives the paint's view and its queued callback runs on freed memory. | On Windows, open a deck with one video several times. |
| 0075 | sw undo | `impl_DoUndoRedo` takes the current view's shell even when that view shows another document; an undo during a second document's import moves the first one's cursor past its nodes. | Load a DOCX with two paragraphs of undefined style while another document is open. |
| 0111 | sd `DrawViewShell::FuPermanent` | A shape command makes its object directly, then returns to the selection tool by an asynchronous dispatch; a press that arrives first goes to the construction tool. | Post `.uno:Connector`, then at once a mouse press on the new object. |
| 0112 | sd `DrawViewShell::MouseMove` | A running action is broken when the position lies outside the window's pixels and the window has no focus; a kit position is in logic units, so a drag lives or dies by `HasFocus()`. | Drag a handle through `setGraphicSelection` while the kit's window is not focused (macOS: about one in ten). |

Kit changes upstream would weigh, not plain defects: 0026 (a new `ITiledRenderable::settleViews` so Impress applies queued pane changes before a command), 0082 (a kit save no longer ends Calc's cell edit), 0098 (the XLSX import waits without yielding under the kit), 0102 (a11y events follow the caret's paragraph, not window focus). 0001, 0007, 0033, 0043, 0044 and 0057 make the kit paint, flush and take focus on desktop macOS and Windows, where upstream does not run it; they are a port, offered whole or not at all.

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

`accesskit_atspi_common` also answers sentence queries here (`85f6faca` and after): a feature for AccessKit to judge, with a splitter that knows eight languages' punctuation, not Unicode's rules.

## gpui (`patches/gpui`, against `gpui-pre` 0.3.11)

`scripts/vendor-gpui.sh` rebuilds `vendor/gpui-pre*` from the crates.io release plus these patches. The defects below show in any gpui app; the fixes go to the gpui these crates repackage.

| Patch | Platform | Defect |
| --- | --- | --- |
| 0001 | all | A GIF or WebP frame that does not decode is skipped and the next is asked for; on a damaged file the decoder may never stop. |
| 0002 | Windows, Linux | Window control areas are hit in paint order: a title bar wins over the caption button drawn on it. |
| 0004 | Windows | The accessibility debug record formats local time each frame; asking for the time zone on every caret blink stalls the main thread. |
| 0005 | Linux | AT-SPI places nodes by the window's screen position, sent once: after a move, assistive positions are stale. |
| 0006 | Linux | The XIM input context is made before the open's reply; fcitx5 under XWayland stops working. |
| 0007 | macOS | The subclassing AccessKit adapter swaps the content view's class (breaks under KVO), keeps a retain cycle, and leaves VoiceOver without the focused control. |
| 0008 | Windows | Dropping a window after `WM_DESTROY` hides and destroys it again; a deferred activation report finds no window. |
| 0009 | Windows | A main-thread task after quit logs an error for the destroyed platform window. |
| 0012 | Windows | Save As keeps a name typed without an extension bare. |
| 0013 | Windows | AccessKit starts from the foreground-window test, which misses a window a click brought forward. |

0003 (the test platform counts quit requests) and 0010 (an app names the language that leads Han layout; else the user's first Han language) add what a client asks for; 0011 (no process AppUserModelID) is Ely's choice for how pinned windows group. They are proposals, not defect reports.
