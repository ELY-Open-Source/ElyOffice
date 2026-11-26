import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import pymupdf

from review import review


class ReviewCopyTest(unittest.TestCase):
    def setUp(self):
        temp = TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.source = Path(temp.name) / "source.pdf"
        self.output = self.source.with_name("review.pdf")
        with pymupdf.open() as doc:
            page = doc.new_page()
            page.insert_text((72, 72), "Payment due within 30 days.")
            page.insert_text((72, 110), "Payment due on delivery.")
            doc.save(self.source)

    def test_review_survives_reopening(self):
        original = self.source.read_bytes()
        review(self.source, self.output, 1, "30 days", "建议改为 45 天")
        self.assertEqual(self.source.read_bytes(), original)
        with pymupdf.open(self.output) as doc:
            page = doc[0]
            mark, = page.annots()
            self.assertEqual(mark.type[1], "Highlight")
            self.assertEqual(mark.info["content"], "建议改为 45 天")
            self.assertTrue(mark.rect.contains(page.search_for("30 days")[0]))
        with self.assertRaises(FileExistsError):
            review(self.source, self.output, 1, "30 days", "second review")

    def test_missing_or_ambiguous_text(self):
        for quote in ["Payment due", "90 days"]:
            with self.subTest(quote=quote), self.assertRaises(ValueError):
                review(self.source, self.output, 1, quote, "Check this")
            self.assertFalse(self.output.exists())
