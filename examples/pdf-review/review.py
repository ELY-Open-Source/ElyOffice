"""Save an exported PDF with a highlight and a reviewer comment."""

import argparse
import os
from pathlib import Path
from tempfile import TemporaryDirectory

import pymupdf


def review(source, destination, page_number, quote, comment):
    if not quote.strip() or len(quote.splitlines()) != 1 or not comment.strip():
        raise ValueError("supply a single-line quote and a non-empty comment")
    with pymupdf.open(source) as doc:
        if not doc.is_pdf or doc.needs_pass:
            raise ValueError("supply an unlocked PDF")
        if not doc.permissions & pymupdf.PDF_PERM_ANNOTATE:
            raise ValueError("this PDF does not permit annotations")
        if doc.get_sigflags() > 0:
            raise ValueError("use an unsigned review copy")
        if not 1 <= page_number <= len(doc):
            raise ValueError("page number is out of range")
        page = doc[page_number - 1]
        hits = page.search_for(quote, quads=True)
        # MuPDF may fold case, join adjacent matches, or join split lines.
        # Require a single quad containing precisely the requested text.
        if len(hits) != 1 or page.get_textbox(hits[0].rect).strip() != quote:
            raise ValueError("quote must match one unique text fragment on the page")
        mark = page.add_highlight_annot(hits)
        mark.set_info(content=comment)
        mark.update()
        with TemporaryDirectory(dir=destination.parent) as directory:
            staged = Path(directory) / "review.pdf"
            doc.save(staged, encryption=pymupdf.PDF_ENCRYPT_KEEP)
            # Publish a complete file and never replace an existing destination.
            os.link(staged, destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--page", type=int, required=True, help="1-based page number")
    parser.add_argument("--quote", required=True)
    parser.add_argument("--comment", required=True)
    args = parser.parse_args()
    try:
        review(args.source, args.destination, args.page, args.quote, args.comment)
    except (OSError, RuntimeError, ValueError) as error:
        parser.exit(1, f"{error}\n")
