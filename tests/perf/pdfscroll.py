#!/usr/bin/env python3
"""ElyPDF scrolling by wheel: frame times per kind of PDF."""

import argparse
import datetime
import os
import random
import re
import sys
import time

import drive
import load
import pdfs
import run
import samples

# Notches and ms between: a steady read, then a flick.
PASSES = (("steady", 60, 50), ("flick", 150, 8))
FONT = b"/Font << /F << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >>"


def write_pdf(path, streams):
    """A PDF of one Helvetica page per content stream."""
    offsets, out = {}, open(path + ".part", "wb")
    out.write(b"%PDF-1.7\n")

    def obj(n, body):
        offsets[n] = out.tell()
        out.write(f"{n} 0 obj\n".encode() + body + b"\nendobj\n")

    kids = " ".join(f"{3 + i * 2} 0 R" for i in range(len(streams)))
    obj(1, b"<< /Type /Catalog /Pages 2 0 R >>")
    obj(2, f"<< /Type /Pages /Kids [{kids}] /Count {len(streams)} >>".encode())
    for i, draw in enumerate(streams):
        obj(3 + i * 2, f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {4 + i * 2} 0 R "
                       f"/Resources << ".encode() + FONT + b" >> >>")
        obj(4 + i * 2, f"<< /Length {len(draw)} >>\nstream\n".encode() + draw + b"\nendstream")
    xref, count = out.tell(), 3 + len(streams) * 2
    out.write(f"xref\n0 {count}\n0000000000 65535 f \n".encode())
    out.write(b"".join(f"{offsets[n]:010d} 00000 n \n".encode() for n in range(1, count)))
    out.write(f"trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    out.close()
    os.replace(path + ".part", path)


def text_page(rng, words):
    """Fifty lines of small text, as a dense report."""
    lines = (" ".join(rng.choice(words) for _ in range(14)) for _ in range(50))
    body = b" T* ".join(f"({line}) Tj".encode() for line in lines)
    return b"BT /F 10 Tf 12 TL 54 740 Td " + body + b" ET"


def vector_page(rng):
    """Three thousand stroked segments, as a plan drawing."""
    segments = (f"{rng.uniform(36, 576):.1f} {rng.uniform(36, 756):.1f} m "
                f"{rng.uniform(36, 576):.1f} {rng.uniform(36, 756):.1f} l" for _ in range(3000))
    return b"0.3 w " + " ".join(segments).encode() + b" S"


def made():
    """The three samples, written once: images, text, vectors."""
    folder = os.path.join(run.ROOT, ".work", "load-samples")
    os.makedirs(folder, exist_ok=True)
    images = os.path.join(folder, "pages.pdf")
    if not os.path.exists(images):
        samples.pdf(images + ".part")
        os.replace(images + ".part", images)
    text, vector = os.path.join(folder, "scroll-text.pdf"), os.path.join(folder, "scroll-vector.pdf")
    words = "local file engine page table slide note draft review commit".split()
    if not os.path.exists(text):
        rng = random.Random(11)
        write_pdf(text, [text_page(rng, words) for _ in range(1000)])
    if not os.path.exists(vector):
        rng = random.Random(13)
        write_pdf(vector, [vector_page(rng) for _ in range(300)])
    return {"images, 1000 pages": images, "text, 1000 pages": text, "vectors, 300 pages": vector}


PARTS = re.compile(r"wait to draw p50 [\d.]+ p95 ([\d.]+) ms; draw end to shown p50 [\d.]+ p95 ([\d.]+) ms; refresh (\S+)")


def parts(lines):
    """Worst wait-to-draw and draw-to-shown p95, and the refresh."""
    found = [m for m in map(PARTS.search, lines) if m]
    if not found:
        return None
    return max(float(m.group(1)) for m in found), max(float(m.group(2)) for m in found), found[-1].group(3)


def scrolled(work, name, path):
    """Each pass's frames and stalls on one PDF."""
    app = run.App("ely-pdf", load.data_home(work, f"scroll-{len(os.listdir(work))}"), [path], frames=True)
    rows = []
    try:
        # macOS drives by AX: no pointer to park, no rectangle.
        mac = sys.platform == "darwin"
        if mac:
            app.wait(r"ui: first frame presented", timeout=30)
        else:
            load.first_place(app)
        app.wait(r"pdf: a view of \d+ pages", timeout=120)
        app.front()
        time.sleep(2.5)
        for kind, notches, pace in PASSES:
            for direction in (1, -1):
                since = len(app.lines())
                run.HOST.scroll(app.process.pid, direction * notches, pace)
                # The frame line flushes every two seconds.
                time.sleep(4.5)
                lines = app.lines()[since:]
                motion = f"{kind} {'down' if direction > 0 else 'up'}"
                rows.append((name, motion, load.frames(lines), load.stalls(lines), parts(lines)))
        # Under Xvfb the window settles a few pixels late.
        if not mac and (why := load.interfered(app, 0)):
            raise load.Interfered(why)
    finally:
        app.stop()
    return rows


def cells(row):
    name, motion, seen, stalls, split = row
    wait, shown, refresh = split or ("", "", "")
    if seen is None:
        return f"| {name} | {motion} | none | | | | | | | {len(stalls)} |"
    frames, lost, late, p95, worst = seen
    return (f"| {name} | {motion} | {frames} ({lost} lost) | {late} | {p95:.1f} | {worst:.1f} | {wait} | {shown} | {refresh} | "
            f"{len(stalls)}, max {max(stalls, default=0)} |")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=os.path.join(run.ROOT, "docs", "perf"))
    args = parser.parse_args()
    work = os.path.join(run.ROOT, ".work", f"S{datetime.datetime.now():%H%M%S}")
    os.makedirs(work)
    run.HOST = drive.host(work)
    pdfs.build()
    rows = []
    for name, path in made().items():
        rows += scrolled(work, name, path)
    problems = [f"{name}, {motion}: {load.frames_failed(seen)}" for name, motion, seen, *_ in rows if load.frames_failed(seen)]
    problems += [f"{name}, {motion}: main thread busy over 100 ms, max {max(stalls)} ms" for name, motion, _, stalls, _ in rows if stalls]
    env = run.HOST.environment()
    lines = [f"# ElyPDF wheel scrolling {datetime.date.today().isoformat()}", "",
             f"{env['machine']}, {env['cpu']}, {env['system']}. Steady: {PASSES[0][1]} notches {PASSES[0][2]} ms apart; "
             f"flick: {PASSES[1][1]} notches {PASSES[1][2]} ms apart. Late is over 8.3 ms dirty to shown.", "",
             "| PDF | Pass | Frames | Late | Dirty to shown p95 ms | Max ms | Wait to draw p95 | Draw end to shown p95 | Refresh ms | Stalls |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |", *map(cells, rows), "", "Problems:", ""]
    lines += [f"- {problem}" for problem in problems] or ["- none"]
    os.makedirs(args.out, exist_ok=True)
    path = os.path.join(args.out, f"{datetime.date.today().isoformat()}-{load.machine()}-pdf-scroll.md")
    with open(path, "w", encoding="utf-8", newline="\n") as record:
        record.write("\n".join(lines) + "\n")
    print(path)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
