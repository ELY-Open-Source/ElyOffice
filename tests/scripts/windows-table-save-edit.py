"""Real window: a save mid-edit keeps the typing (R15.39)."""
import os
import subprocess
import sys
import tempfile
import time
import zipfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "perf"))
import drive
import run

if sys.platform != "win32":
    sys.exit(print("SKIP: real keys through the Windows driver"))
for args in (["-p", "ely-lok", "--bin", "ely-lok-worker"], ["-p", "ely-table"]):
    subprocess.run(["cargo", "build", "-q", *args, "--manifest-path", os.path.join(drive.ROOT, "Cargo.toml")], check=True)
exe = os.path.join(os.environ.get("CARGO_TARGET_DIR", os.path.join(drive.ROOT, "target")), "debug", "ely-table.exe")
if not os.path.isfile(exe):
    sys.exit(f"FAIL: no build at {exe}")
work = tempfile.mkdtemp()
run.HOST = drive.host(work)
run.HOST.exe = lambda slug, frames: exe
book = os.path.join(work, "book.xlsx")
NS = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" ' \
     'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
with zipfile.ZipFile(book, "w") as archive:
    archive.writestr("[Content_Types].xml", '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                     '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                     '<Default Extension="xml" ContentType="application/xml"/>'
                     '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
                     '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>')
    archive.writestr("_rels/.rels", '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                     '<Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>')
    archive.writestr("xl/_rels/workbook.xml.rels", '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                     '<Relationship Id="s1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>')
    archive.writestr("xl/workbook.xml", f'<workbook {NS}><sheets><sheet name="Data" sheetId="1" r:id="s1"/></sheets></workbook>')
    archive.writestr("xl/worksheets/sheet1.xml", f'<worksheet {NS}><sheetData><row r="1"><c r="A1"><v>7</v></c></row></sheetData></worksheet>')


def ask(app, *words):
    out = subprocess.run([exe, "cell", *words], env=app.env, capture_output=True, text=True, timeout=60,
                         creationflags=drive.QUIET)
    if out.returncode:
        sys.exit(f"FAIL: cell {words[0]} exited {out.returncode}: {out.stderr.strip()}")
    return out.stdout


def shown(app, text):
    """Waits until a text field shows `text`; what they show."""
    for _ in range(50):
        values = run.HOST.values(app.process.pid)
        if text in values:
            return values
        time.sleep(0.1)
    sys.exit(f"FAIL: the bars show {values}, not {text!r}")


def disk():
    """Every XML part of the saved book, joined."""
    with zipfile.ZipFile(book) as archive:
        return "".join(archive.read(name).decode("utf-8") for name in archive.namelist() if name.endswith(".xml"))


app = run.App("ely-table", os.path.join(work, "home"), [book])
try:
    app.wait(r"lok: first tile drawn", timeout=120)
    time.sleep(2)
    app.front()
    run.HOST.type(app.process.pid, "abc")
    shown(app, "abc")
    since = len(app.lines())
    ask(app, "save", book)
    app.wait(r"office: saved .*book\.xlsx", since=since, timeout=30)
    saved = disk()
    if "abc" in saved or "<v>7</v>" not in saved:
        sys.exit("FAIL: the save mid-edit wrote other than the cell as entered")
    run.HOST.type(app.process.pid, "def")
    shown(app, "abcdef")
    run.HOST.enter(app.process.pid)
    time.sleep(1)
    since = len(app.lines())
    ask(app, "save", book)
    app.wait(r"office: saved .*book\.xlsx", since=since, timeout=30)
    if "abcdef" not in disk():
        sys.exit("FAIL: the entered cell is not on disk")
    live = ask(app, "get", book, "Data!A1")
    if '"abcdef"' not in live:
        sys.exit(f"FAIL: the open book holds {live.strip()}")
finally:
    app.stop()
print("ok: a save mid cell edit kept the typing; Enter then saved it whole")
