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

# A tall page: its middle row meets the first line.
python -I - "$(cygpath -w "$work/probe.pdf")" <<'EOF'
import sys
stream = b"BT /F1 24 Tf 36 940 Td (Hello PDF reader) Tj ET BT /F1 24 Tf 36 100 Td (second line) Tj ET"
objects = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 1000] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream",
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
out, offsets = bytearray(b"%PDF-1.4\n"), []
for n, body in enumerate(objects, 1):
    offsets.append(len(out))
    out += b"%d 0 obj\n" % n + body + b"\nendobj\n"
xref = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
out += b"".join(b"%010d 00000 n \n" % at for at in offsets)
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%EOF\n" % (len(objects) + 1, xref)
open(sys.argv[1], "wb").write(out)
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
function Drag($from, $to, $y) {
    [Mouse]::SetCursorPos($from, $y) | Out-Null
    Start-Sleep -Milliseconds 200
    [Mouse]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    foreach ($step in 1..10) {
        [Mouse]::SetCursorPos([int]($from + ($to - $from) * $step / 10), $y) | Out-Null
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
    } else {
        Read-State 'page'
        $box = $view.Current.BoundingRectangle
        $y = [int]($box.Top + $box.Height / 2)
        Drag ([int]($box.Left + $box.Width * 0.1)) ([int]($box.Left + $box.Width * 0.9)) $y
        Read-State 'picked'
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
cat >"$work/want.log" <<'EOF'
caret|ControlType.Edit|Hello 世界 😀 end|
line|ControlType.Edit|Hello 世界 😀 end|Hello 世界 😀 end
emoji|ControlType.Edit|Hello 世界 😀 end|😀
end|ControlType.Edit|Hello 世界 😀 end|end
page|ControlType.Document|Hello PDF reader/second line/|
picked|ControlType.Document|Hello PDF reader/second line/|Hello PDF reader
EOF
cat "$work/doc.log" "$work/pdf.log" | tr -d '\r' | diff "$work/want.log" - || { echo "FAIL: the reader saw other text" >&2; exit 1; }
echo "PASS: ElyDoc's paragraph and ElyPDF's page, with caret and selection, reach UI Automation"
