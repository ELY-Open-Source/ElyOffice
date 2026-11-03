#!/usr/bin/env python3
# PERF-10's read-back and log parsing on made-up files.
import os
import shutil
import tempfile
import unittest

import load
import loadfiles


class ReadBack(unittest.TestCase):
    def setUp(self):
        self.work = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.work)

    def test_a_missing_edit_is_named(self):
        path = os.path.join(self.work, "doc-002-small.docx")
        loadfiles.small_docx(path, 2)
        self.assertEqual(load.saved("ely-doc", path, 0), [])
        self.assertEqual(load.saved("ely-doc", path, 3), ["doc-002-small.docx: saved tokens [0], not [3]"])

    def test_stale_links_fail_and_kept_links_pass(self):
        loadfiles.sources(self.work)
        path = os.path.join(self.work, "doc-001-linked.docx")
        loadfiles.linked_docx(path, self.work)
        links = loadfiles.word_links(path)
        self.assertEqual(links, {"INCLUDETEXT field kept": True, "INCLUDETEXT shows the source's text": False,
                                 "linked picture kept": True, "OLE link kept": True})
        book = os.path.join(self.work, "table-000-big.xlsx")
        loadfiles.workbook(book, 40, linked=True)
        self.assertEqual(loadfiles.book_links(book), {"external book kept": True, "Links!A1 formula kept": True,
                                                      "Links!A1 shows the source's value": False})

    def test_an_unsaved_cell_is_named(self):
        path = os.path.join(self.work, "table-003-small.xlsx")
        loadfiles.workbook(path, 20)
        problems = load.saved("ely-table", path, 2)
        self.assertIn("table-003-small.xlsx: Data!H1 saved None, not 2", problems)
        self.assertIn("table-003-small.xlsx: Data!A1 saved 7, not 1002", problems)

    def test_typed_keys_must_all_be_there(self):
        path = os.path.join(self.work, "front.md")
        with open(path, "w", encoding="utf-8") as note:
            note.write("A paragraphxxxx.\n")
        self.assertTrue(load.typed("ely-mark", path, 4))
        self.assertFalse(load.typed("ely-mark", path, 5))


class Logs(unittest.TestCase):
    def test_frame_lines_add_up(self):
        lines = ["[x] ui: frames 120 (lost 0): draw p50 1.0 p95 2.0 max 3.0 ms; "
                 "dirty to shown p50 4.0 p95 9.5 max 20.0 ms; over 8.3 ms 7",
                 "[x] ui: frames 30 (lost 1): draw p50 1.0 p95 2.0 max 3.0 ms; "
                 "dirty to shown p50 4.0 p95 5.0 max 6.0 ms; over 8.3 ms 0"]
        self.assertEqual(load.frames(lines), (150, 1, 7, 9.5, 20.0))
        self.assertIsNone(load.frames(["[x] lok: edit to tile 5 ms"]))

    def test_zoom_steps_are_judged(self):
        self.assertEqual(load.zoomed(["[x] lok: zoom 50%", "[x] lok: zoom 125%"]), 125)
        self.assertIsNone(load.zoomed(["[x] lok: edit to tile 5 ms"]))
        self.assertTrue(load.reached(100, 25, "minus", 4))
        self.assertTrue(load.reached(100, 400, "plus", 5))
        self.assertTrue(load.reached(50, 25, "minus", 4))
        self.assertFalse(load.reached(100, 100, "plus", 1))
        self.assertTrue(load.reached(100, 132, "0", 2))
        self.assertFalse(load.reached(100, 100, "0", -3))
        self.assertFalse(load.reached(None, 100, "plus", 1))

    def test_stalls_are_read(self):
        self.assertEqual(load.stalls(["[x] ui: the main thread was busy 140 ms", "[x] other"]), [140])


if __name__ == "__main__":
    unittest.main()
