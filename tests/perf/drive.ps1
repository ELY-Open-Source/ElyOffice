# Window input for bench.py on Windows, through UI Automation.
param([string]$Command, [int]$Id, [string]$Arg = '')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Input {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern short VkKeyScan(char c);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, IntPtr pid);
    [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr window);
    [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
    [DllImport("user32.dll", EntryPoint = "GetWindowThreadProcessId")] public static extern uint Owner(IntPtr window, out uint pid);
    // The foreground's input queue lets another take the front.
    public static bool Front(IntPtr window) {
        uint ours = GetCurrentThreadId(), theirs = GetWindowThreadProcessId(GetForegroundWindow(), IntPtr.Zero);
        bool joined = theirs != 0 && theirs != ours && AttachThreadInput(ours, theirs, true);
        try {
            BringWindowToTop(window);
            SetForegroundWindow(window);
        } finally {
            if (joined) { AttachThreadInput(ours, theirs, false); }
        }
        return GetForegroundWindow() == window;
    }
}
"@
# Screen pixels, as UI Automation gives them.
[Input]::SetProcessDPIAware() | Out-Null
$A = [System.Windows.Automation.AutomationElement]

function Window {
    $owner = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, $Id)
    $window = $A::RootElement.FindFirst('Children', $owner)
    if (-not $window) { throw "no window for process $Id" }
    $window
}
# Any of the app's windows: a dialog is its own.
function Named([string]$name) {
    $owner = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, $Id)
    $named = New-Object System.Windows.Automation.PropertyCondition($A::NameProperty, $name)
    foreach ($window in $A::RootElement.FindAll('Children', $owner)) {
        $found = $window.FindFirst('Descendants', $named)
        if ($found) { return $found }
    }
}
function Mine { $pid0 = [uint32]0; [Input]::Owner([Input]::GetForegroundWindow(), [ref]$pid0) | Out-Null; $pid0 -eq $Id }
# Input goes to the front window: it must be the app's.
function Ours {
    if (Mine) { return }
    $handle = [IntPtr](Window).Current.NativeWindowHandle
    # A console flashing up takes the front; take it back once.
    for ($try = 0; $try -lt 5 -and -not [Input]::Front($handle); $try++) { Start-Sleep -Milliseconds 100 }
    if (Mine) { [Console]::Error.WriteLine("drive: took the front back"); return }
    throw "process $Id lost the front to $(Holder); no input sent"
}
# The process owning the front window, by name and id.
function Holder {
    $pid0 = [uint32]0
    [Input]::Owner([Input]::GetForegroundWindow(), [ref]$pid0) | Out-Null
    "$((Get-Process -Id $pid0 -ErrorAction SilentlyContinue).ProcessName) ($pid0)"
}
# SendKeys' journal hook drops keys on Windows 11.
function Key([byte]$vk, [bool]$up) {
    if (-not $up) { Ours }
    [Input]::keybd_event($vk, 0, $(if ($up) { 2 } else { 0 }), [UIntPtr]::Zero)
}
function Tap([byte]$vk) { Key $vk $false; Key $vk $true }
function ClickAt([int]$x, [int]$y) {
    [Input]::SetCursorPos($x, $y) | Out-Null
    [Input]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    [Input]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
}
function Click($element) {
    Ours
    $r = $element.Current.BoundingRectangle
    ClickAt ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2))
}

switch ($Command) {
    'lingering' {
        # This script's own command line holds the marker too.
        $found = Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -and $_.CommandLine.Contains($Arg) }
        if ($found) { 'yes' } else { 'no' }
    }
    'front' {
        $window = Window
        $handle = [IntPtr]$window.Current.NativeWindowHandle
        # Never a click: the title strip holds the app's controls.
        for ($try = 0; $try -lt 5 -and -not [Input]::Front($handle); $try++) { Start-Sleep -Milliseconds 200 }
        Start-Sleep -Milliseconds 200
        if ([Input]::GetForegroundWindow() -ne $handle) { throw "process $Id did not come to the front; $(Holder) holds it" }
    }
    'type' {
        foreach ($c in $Arg.ToCharArray()) {
            $scan = [Input]::VkKeyScan($c)
            $shift = ($scan -band 0x100) -ne 0
            if ($shift) { Key 0x10 $false }
            Tap ([byte]($scan -band 0xff))
            if ($shift) { Key 0x10 $true }
            Start-Sleep -Milliseconds 20
        }
    }
    'enter' { Tap 0x0D }
    'chord' {
        Key 0x11 $false
        Tap ([byte][char]$Arg.ToUpper())
        Key 0x11 $true
    }
    'press' {
        $element = Named $Arg
        if (-not $element) { throw "no control named $Arg" }
        $invoke = $null
        if ($element.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$invoke)) { $invoke.Invoke() } else { Click $element }
    }
    'shows' { if (Named $Arg) { 'yes' } else { 'no' } }
    'field' {
        $edit = New-Object System.Windows.Automation.PropertyCondition($A::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)
        $field = (Window).FindFirst('Descendants', $edit)
        if (-not $field) { throw 'no text field' }
        Click $field
    }
    'machine' {
        $cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1)
        $os = Get-CimInstance Win32_OperatingSystem
        $memory = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
        $gpu = (Get-CimInstance Win32_VideoController | ForEach-Object { $_.Name }) -join ', '
        "$($cpu.Name.Trim())|$($cpu.NumberOfLogicalProcessors)|$memory|$gpu|$($os.Caption) $($os.Version)"
    }
    default { throw "unknown command $Command" }
}
