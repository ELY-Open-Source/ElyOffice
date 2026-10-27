#!/usr/bin/env bash
# Screen readers reach ElyDoc's and ElyPDF's text and selection.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) ;; *) echo "SKIP: not Windows"; exit 0 ;; esac
[ -n "${ELY_LOK_DIR:-}" ] || [ -d "$root/engines/lo-src/instdir/program" ] || { echo "SKIP: no LOK build"; exit 0; }
[ -n "${ELY_MUPDF_DIR:-}" ] || [ -d "$root/engines/mupdf-build" ] || { echo "SKIP: no MuPDF build"; exit 0; }
export MSYS2_ARG_CONV_EXCL='*'

work=$root/target/a11y-test
stop() { taskkill /IM ely-doc.exe /F >/dev/null 2>&1 || true; taskkill /IM ely-pdf.exe /F >/dev/null 2>&1 || true; }
trap stop EXIT
stop
rm -rf "$work"
mkdir -p "$work/data"
(cd "$root" && cargo build -q -p ely-doc -p ely-lok -p ely-pdf -p ely-pdf-engine --bins)

cat >"$work/probe.fodt" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Hello 世界 😀 end</text:p></office:text></office:body>
</office:document>
EOF

# A tall page, then three short ones.
python -I - "$(cygpath -w "$work")" <<'EOF'
import os, sys
def line(y, text):
    return b"BT /F1 24 Tf 36 %d Td (%s) Tj ET " % (y, text)
def write(name, size, streams):
    kids = b" ".join(b"%d 0 R" % (4 + 2 * n) for n in range(len(streams)))
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [%s] /Count %d >>" % (kids, len(streams)),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    for n, stream in enumerate(streams):
        objects.append(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %d %d] /Contents %d 0 R /Resources << /Font << /F1 3 0 R >> >> >>" % (size[0], size[1], 5 + 2 * n))
        objects.append(b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream")
    out, offsets = bytearray(b"%PDF-1.4\n"), []
    for n, body in enumerate(objects, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % n + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
    out += b"".join(b"%010d 00000 n \n" % at for at in offsets)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%EOF\n" % (len(objects) + 1, xref)
    open(os.path.join(sys.argv[1], name), "wb").write(out)
write("probe.pdf", (300, 1000), [line(940, b"Hello PDF reader") + line(100, b"second line")])
write("pages.pdf", (300, 120), [line(50, b"page " + word) for word in (b"one", b"two", b"three")])
EOF

# UI Automation is what NVDA reads.
cat >"$work/read.ps1" <<'EOF'
param([string]$Kind, [string]$Exe, [string]$File, [string]$Data, [string]$Out)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Mouse {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
"@
# Screen pixels, as UI Automation gives them.
[Mouse]::SetProcessDPIAware() | Out-Null
$A = [System.Windows.Automation.AutomationElement]
$T = [System.Windows.Automation.TextPattern]
$env:ELY_DATA_ROOT = $Data
$env:ELY_NO_DIALOG = '1'
$app = Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru
$owner = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, $app.Id)
$texts = New-Object System.Windows.Automation.PropertyCondition($A::IsTextPatternAvailableProperty, $true)
function Find-Text {
    foreach ($i in 1..120) {
        $window = $A::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $owner)
        if ($window) {
            $found = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $texts)
            if ($found) { return $found }
        }
        Start-Sleep -Milliseconds 500
    }
    throw 'no element offers a text pattern'
}
$log = New-Object System.Collections.Generic.List[string]
function Read-State($label) {
    Start-Sleep -Milliseconds 1500
    $edit = Find-Text
    $pattern = $edit.GetCurrentPattern($T::Pattern)
    $chosen = ($pattern.GetSelection() | ForEach-Object { $_.GetText(-1) }) -join '+'
    $text = $pattern.DocumentRange.GetText(-1).Replace("`r", '').Replace("`n", '/')
    $log.Add("$label|$($edit.Current.ControlType.ProgrammaticName)|$text|$($chosen.Replace("`n", '/'))")
}
function Drag($box, $x0, $y0, $x1, $y1) { # in widths of $box
    $at = { param($x, $y) [Mouse]::SetCursorPos([int]($box.Left + $box.Width * $x), [int]($box.Top + $box.Width * $y)) | Out-Null }
    & $at $x0 $y0
    Start-Sleep -Milliseconds 200
    [Mouse]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    foreach ($step in 1..10) {
        & $at ($x0 + ($x1 - $x0) * $step / 10) ($y0 + ($y1 - $y0) * $step / 10)
        Start-Sleep -Milliseconds 50
    }
    [Mouse]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
}
try {
    $view = Find-Text
    $view.SetFocus()
    Start-Sleep -Milliseconds 800
    if ($Kind -eq 'doc') {
        [System.Windows.Forms.SendKeys]::SendWait('^{HOME}')
        Read-State 'caret'
        [System.Windows.Forms.SendKeys]::SendWait('+{END}')
        Read-State 'line'
        [System.Windows.Forms.SendKeys]::SendWait('{HOME}{RIGHT 9}+{RIGHT}')
        Read-State 'emoji'
        [System.Windows.Forms.SendKeys]::SendWait('{RIGHT}+{RIGHT 4}')
        Read-State 'end'
    } elseif ($Kind -eq 'pdf') {
        Read-State 'page'
        $box = $view.Current.BoundingRectangle
        $y = $box.Height / 2 / $box.Width
        Drag $box 0.1 $y 0.9 $y
        Read-State 'picked'
    } else {
        # At fit width a page is 0.4 widths tall.
        Drag $view.Current.BoundingRectangle 0.1 0.2 0.9 0.6
        Read-State 'across'
        [System.Windows.Forms.SendKeys]::SendWait('{END}')
        Read-State 'scrolled'
    }
} finally {
    [System.IO.File]::WriteAllLines($Out, $log)
    Stop-Process -Id $app.Id -Force
}
EOF

w() { cygpath -w "$1"; }
read_app() { # kind exe file
  powershell -NoProfile -ExecutionPolicy Bypass -File "$(w "$work/read.ps1")" -Kind "$1" -Exe "$(w "$root/target/debug/$2.exe")" \
    -File "$(w "$work/$3")" -Data "$(w "$work/data")" -Out "$(w "$work/$1.log")"
}
read_app doc ely-doc probe.fodt
read_app pdf ely-pdf probe.pdf
read_app pages ely-pdf pages.pdf
cat >"$work/want.log" <<'EOF'
caret|ControlType.Edit|Hello 世界 😀 end|
line|ControlType.Edit|Hello 世界 😀 end|Hello 世界 😀 end
emoji|ControlType.Edit|Hello 世界 😀 end|😀
end|ControlType.Edit|Hello 世界 😀 end|end
page|ControlType.Document|Hello PDF reader/second line/|
picked|ControlType.Document|Hello PDF reader/second line/|Hello PDF reader
across|ControlType.Document|page one/page two/|page one/page two
scrolled|ControlType.Document|page two/page three/|page two
EOF
cat "$work/doc.log" "$work/pdf.log" "$work/pages.log" | tr -d '\r' | diff "$work/want.log" - || { echo "FAIL: the reader saw other text" >&2; exit 1; }
echo "PASS: ElyDoc's paragraph and ElyPDF's page, with caret and selection, reach UI Automation"
