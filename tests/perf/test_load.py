#!/usr/bin/env python3
# PERF-10's read-back and log parsing on made-up files.
import os
import shutil
import tempfile
import unittest

import feedback
import load
import loadfiles
import richfiles


class ReadBack(unittest.TestCase):
    def setUp(self):
        self.work = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.work)

    def test_a_missing_edit_is_named(self):
        path = os.path.join(self.work, "doc-002-small.docx")
        loadfiles.small_docx(path, 2)
        self.assertEqual(richfiles.saved("ely-doc", path, 0), [])
        self.assertEqual(richfiles.saved("ely-doc", path, 3), ["doc-002-small.docx: saved tokens [0], not [3]"])

    def test_stale_links_fail_and_kept_links_pass(self):
        loadfiles.sources(self.work)
        path = os.path.join(self.work, "doc-001-linked.docx")
        loadfiles.linked_docx(path, self.work)
        links = loadfiles.word_links(path)
        self.assertEqual(links, {"INCLUDETEXT field kept": True, "INCLUDETEXT shows the text it should": False,
                                 "linked picture kept": True, "OLE link kept": True})
        book = os.path.join(self.work, "table-000-big.xlsx")
        loadfiles.workbook(book, 40, linked=True)
        self.assertEqual(loadfiles.book_links(book), {"external book kept": True, "Links!A1 formula kept": True,
                                                      "Links!A1 shows the value it should": False})
        self.assertEqual(richfiles.saved("ely-doc", path, 0), [])

    def test_an_unsaved_cell_is_named(self):
        path = os.path.join(self.work, "table-003-small.xlsx")
        loadfiles.workbook(path, 20)
        problems = richfiles.saved("ely-table", path, 2)
        self.assertIn("table-003-small.xlsx: Data!H1 saved None, not 2", problems)
        self.assertIn("table-003-small.xlsx: Data!A1 saved 7, not 1002", problems)

    def test_lost_content_is_named(self):
        note = os.path.join(self.work, "mark-000-big.md")
        with open(note, "w", encoding="utf-8") as only:
            only.write("ELYEDIT3")
        self.assertEqual(richfiles.saved("ely-mark", note, 3), ["mark-000-big.md: the note is as written failed"])
        big = os.path.join(self.work, "doc-000-big.docx")
        loadfiles.big_docx(big)
        self.assertEqual(richfiles.saved("ely-doc", big, 0), [])
        loadfiles.rewrite(big, {"word/document.xml": lambda xml: xml.replace(" DATE ", " TIME ")})
        self.assertEqual(richfiles.saved("ely-doc", big, 0), ["doc-000-big.docx: DATE field kept failed"])

    def test_formulas_must_be_recalculated(self):
        book = os.path.join(self.work, "table-004-small.xlsx")
        loadfiles.workbook(book, 500)
        kept = richfiles.book_kept(book, 500, 0)
        formulas = [ok for what, ok in kept.items() if " formula " in what]
        self.assertTrue(formulas and all(formulas))
        self.assertFalse(any(ok for what, ok in kept.items() if " formula " not in what), "nothing computed yet")
        loadfiles.rewrite(book, {"xl/worksheets/sheet2.xml": lambda xml: xml.replace("<f>SUMIFS(", "<f>0+SUMIFS(", 1)})
        changed = [what for what, ok in richfiles.book_kept(book, 500, 0).items() if " formula " in what and not ok]
        self.assertEqual(len(changed), 1)
        self.assertTrue(changed[0].startswith("Calc!A2 formula SUMIFS("))

    def test_an_error_value_is_a_failed_cell(self):
        book = os.path.join(self.work, "table-005-small.xlsx")
        loadfiles.workbook(book, 500)
        loadfiles.rewrite(book, {"xl/worksheets/sheet2.xml": lambda xml: xml.replace(
            '<c r="A2"><f>', '<c r="A2" t="e"><v>#VALUE!</v><f>', 1)})
        self.assertIn("table-005-small.xlsx: Calc!A2 recalculated to -3275 failed", richfiles.saved("ely-table", book, 0))

    def test_error_values_fail_links_and_keys(self):
        loadfiles.sources(self.work)
        book = os.path.join(self.work, "table-000-big.xlsx")
        loadfiles.workbook(book, 40, linked=True, typing=True)
        loadfiles.rewrite(book, {"xl/worksheets/sheet4.xml": lambda xml: xml.replace(
            '<c r="A1"><f>', '<c r="A1" t="e"><f>', 1).replace("<v>1.01</v>", "<v>#VALUE!</v>", 1)})
        self.assertFalse(loadfiles.book_links(book, loadfiles.RATES_CACHED[0])["Links!A1 shows the value it should"])
        loadfiles.rewrite(book, {"xl/worksheets/sheet1.xml": lambda xml: xml.replace(
            "<sheetData></sheetData>", '<sheetData><row r="1"><c r="A1" t="e"><v>#VALUE!</v></c></row></sheetData>', 1)})
        self.assertFalse(richfiles.typed("ely-table", book, 3))

    def test_a_big_book_keeps_its_saved_link_value(self):
        loadfiles.sources(self.work)
        book = os.path.join(self.work, "table-000-big.xlsx")
        loadfiles.workbook(book, 40, linked=True, typing=True)
        links = lambda: [p for p in richfiles.saved("ely-table", book, 0) if "Links!" in p]
        self.assertEqual(links(), [])
        loadfiles.rewrite(book, {"xl/worksheets/sheet4.xml": lambda xml: xml.replace(
            f"<v>{loadfiles.RATES_CACHED[0]}</v>", f"<v>{loadfiles.RATES_NOW[0]}</v>", 1)})
        self.assertEqual(links(), ["table-000-big.xlsx: Links!A1 shows the value it should failed"])

    def test_typed_keys_must_all_be_there(self):
        path = os.path.join(self.work, "front.md")
        with open(path, "w", encoding="utf-8") as note:
            note.write("A paragraphxxxx.\n")
        self.assertTrue(richfiles.typed("ely-mark", path, 4))
        self.assertFalse(richfiles.typed("ely-mark", path, 5))


class Logs(unittest.TestCase):
    def test_frame_lines_add_up(self):
        lines = ["[x] ui: frames 120 (lost 0): draw p50 1.0 p95 2.0 max 3.0 ms; "
                 "dirty to shown p50 4.0 p95 9.5 max 20.0 ms; over 8.3 ms 7",
                 "[x] ui: frames 30 (lost 1): draw p50 1.0 p95 2.0 max 3.0 ms; "
                 "dirty to shown p50 4.0 p95 5.0 max 6.0 ms; over 8.3 ms 0"]
        self.assertEqual(load.frames(lines), (150, 1, 7, 9.5, 20.0))
        self.assertIsNone(load.frames(["[x] lok: edit 3 text to tile 5 ms"]))

    def test_zoom_steps_are_exact(self):
        self.assertEqual(load.zooms(["[x] lok: zoom 50%", "[x] lok: edit 3 text to tile 5 ms", "[x] lok: zoom 125%"]), [50, 125])
        self.assertEqual(load.walk(0, 0), [])
        self.assertEqual(load.walk(2, 0), [50, 67])
        self.assertEqual(load.walk(9, 0)[-1], 400)
        self.assertEqual(load.walk(0, 2), [50])
        self.assertEqual(load.walk(5, -3), [50, 67, 80, 100, 125, 100])

    def test_frames_are_a_verdict(self):
        self.assertIsNotNone(load.frames_failed(None))
        self.assertIsNotNone(load.frames_failed((10, 0, 9, 20.0, 150.0)))
        self.assertIsNone(load.frames_failed((10, 0, 9, 20.0, 62.0)))

    def test_typed_keys_are_timed_or_never_drew(self):
        lines = ["[x] lok: edit 4 text to tile 12 ms", "[x] lok: edit 5 .uno:Bold to tile 3 ms",
                 "[x] lok: edit 6 text.replace never drawn: the view is hidden", "[x] lok: edit 7 key to tile 30 ms"]
        self.assertEqual(feedback.answers(lines, feedback.TILE), 2, "only typed keys answer")
        with self.assertRaisesRegex(RuntimeError, "edit 8 key never drawn: it changed no tile"):
            feedback.answers(lines + ["[x] lok: edit 8 key never drawn: it changed no tile"], feedback.TILE)

    def test_each_typed_input_has_one_timing(self):
        typed = ["[x] lok: input 3 text waited 0, took 1 ms", "[x] lok: input 4 text waited 0, took 1 ms"]
        both = typed + ["[x] lok: edit 3 text to tile 12 ms", "[x] lok: edit 4 text to tile 30 ms"]
        self.assertEqual(feedback.timed(both), [12, 30])
        with self.assertRaisesRegex(RuntimeError, "never drew"):
            feedback.timed(both[:3] + ["[x] lok: edit 4 text never drawn: it changed no tile"])
        with self.assertRaisesRegex(RuntimeError, "gave timings"):
            feedback.timed(typed)
        with self.assertRaisesRegex(RuntimeError, "gave timings"):
            feedback.timed(both + ["[x] lok: edit 4 text to tile 31 ms"])
        with self.assertRaisesRegex(RuntimeError, "gave timings"):
            feedback.timed([])

    def test_stalls_are_read(self):
        self.assertEqual(load.stalls(["[x] ui: the main thread was busy 140 ms", "[x] other"]), [140])


if __name__ == "__main__":
    unittest.main()
