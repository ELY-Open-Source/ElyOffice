#!/usr/bin/env bash
# Seven apps name every control UI Automation reaches.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) ;; *) echo "SKIP: not Windows"; exit 0 ;; esac
[ -n "${ELY_LOK_DIR:-}" ] || [ -d "$root/engines/lo-src/instdir/program" ] || { echo "SKIP: no LOK build"; exit 0; }
[ -n "${ELY_MUPDF_DIR:-}" ] || [ -d "$root/engines/mupdf-build" ] || { echo "SKIP: no MuPDF build"; exit 0; }

apps="ely-doc ely-table ely-show ely-pdf ely-mail ely-tree ely-mark"
work=$root/target/a11y-names
stop() { for app in $apps; do taskkill /IM "$app.exe" /F >/dev/null 2>&1 || true; done; }
trap stop EXIT
stop
rm -rf "$work"
mkdir -p "$work"
mail="path+file:///$(cygpath -m "$root/apps/ely-mail")"
(cd "$root" && cargo build -q -p ely-doc -p ely-table -p ely-show -p ely-pdf -p "$mail" -p ely-tree -p ely-mark \
  -p ely-lok -p ely-pdf-engine --bins)

. "$root/tests/scripts/a11y-samples.sh"
a11y_samples "$work"
export MSYS2_ARG_CONV_EXCL='*'

# What NVDA reads, through UI Automation.
cat >"$work/walk.ps1" <<'EOF'
param([string]$Exe, [string]$File, [string]$Data, [string]$Out)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Mouse {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
}
"@
# Screen pixels, as UI Automation gives them.
[Mouse]::SetProcessDPIAware() | Out-Null
$A = [System.Windows.Automation.AutomationElement]
$env:ELY_DATA_ROOT = $Data
$env:ELY_NO_DIALOG = '1'
$app = if ($File) { Start-Process -FilePath $Exe -ArgumentList "`"$File`"" -PassThru } else { Start-Process -FilePath $Exe -PassThru }
$owner = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, $app.Id)
$acts = 'Button','CheckBox','ComboBox','Edit','Hyperlink','ListItem','MenuItem','RadioButton','Slider','Spinner','TabItem','TreeItem','SplitButton','DataItem','Document'
$log = New-Object System.Collections.Generic.List[string]
function Kind($e) { $e.Current.ControlType.ProgrammaticName -replace 'ControlType\.', '' }
# SendKeys' journal hook drops keys on Windows 11.
function Press([byte]$key) {
    [Mouse]::keybd_event($key, 0, 0, [UIntPtr]::Zero)
    [Mouse]::keybd_event($key, 0, 2, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 300
    $A::FocusedElement
}
try {
    $window = $null
    foreach ($i in 1..120) {
        $window = $A::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $owner)
        if ($window) { break }
        Start-Sleep -Milliseconds 500
    }
    if (-not $window) { throw 'no window' }
    Start-Sleep -Seconds 6
    $all = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($e in $all) {
        if ($acts -contains (Kind $e) -and -not $e.Current.Name -and -not $e.Current.LabeledBy) {
            $log.Add("unnamed $(Kind $e) at $($e.Current.BoundingRectangle)")
        }
    }
    $pages = @($all | Where-Object { (Kind $_) -eq 'Document' }).Count
    $reached = $false
    # A click on the title bar's empty middle activates it.
    $menus = New-Object System.Windows.Automation.PropertyCondition($A::ControlTypeProperty, [System.Windows.Automation.ControlType]::MenuBar)
    $bar = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $menus).Current.BoundingRectangle
    $box = $window.Current.BoundingRectangle
    [Mouse]::SetCursorPos([int]($box.Left + $box.Width * 0.6), [int]($bar.Top + $bar.Height / 2)) | Out-Null
    [Mouse]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    [Mouse]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 500
    if ($A::FocusedElement.Current.ProcessId -ne $app.Id) { throw 'the app never came to the front' }
    $seen = @{}
    $last = ''
    $stops = 0
    foreach ($i in 1..150) {
        $focused = Press 0x09
        $key = $focused.GetRuntimeId() -join '.'
        # A document keeps Tab; F6 leaves it.
        if ($key -eq $last) {
            $focused = Press 0x75
            $key = $focused.GetRuntimeId() -join '.'
            if ($key -eq $last) { $log.Add("trapped in $(Kind $focused) [$($focused.Current.Name)]"); break }
        }
        if ($focused.Current.ProcessId -ne $app.Id) { throw "focus left the app at stop $stops" }
        if ($seen.ContainsKey($key)) { break }
        $seen[$key] = 1
        $last = $key
        $stops++
        if (-not $focused.Current.Name) { $log.Add("nameless stop $stops $(Kind $focused)") }
        for ($up = $focused; $up -and -not $reached; $up = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($up)) {
            $reached = (Kind $up) -eq 'Document'
        }
    }
    # Tab must lead back to the page.
    if ($pages -and -not $reached) { $log.Add('unreached document') }
    $log.Add("stops $stops")
} catch { $log.Add("error $_") } finally {
    [System.IO.File]::WriteAllLines($Out, $log)
    Stop-Process -Id $app.Id -Force
    $app.WaitForExit()
}
EOF

w() { cygpath -w "$1"; }
failed=0
for spec in "ely-doc doc.fodt" "ely-table table.fods" "ely-show show.fodp" "ely-pdf pdf.pdf" "ely-mail -" "ely-tree repo" "ely-mark mark.md"; do
  set -- $spec
  mkdir -p "$work/data-$1"
  file=""
  [ "$2" = - ] || file=$(w "$work/$2")
  powershell -NoProfile -ExecutionPolicy Bypass -File "$(w "$work/walk.ps1")" -Exe "$(w "$root/target/debug/$1.exe")" \
    -File "$file" -Data "$(w "$work/data-$1")" -Out "$(w "$work/$1.log")"
  log=$(tr -d '\r' <"$work/$1.log")
  stops=$(printf '%s\n' "$log" | sed -n 's/^stops //p')
  if printf '%s\n' "$log" | grep -qE '^(unnamed|nameless|unreached|trapped|error)' || [ "${stops:-0}" -lt 3 ]; then
    echo "FAIL: $1" >&2
    printf '%s\n' "$log" >&2
    failed=1
  else
    echo "ok: $1, $stops Tab stops"
  fi
done
[ "$failed" = 0 ] || exit 1
echo "PASS: seven apps name every control and every Tab stop; Tab reaches each document, F6 leaves it"
