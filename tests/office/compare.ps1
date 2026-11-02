# Usage: compare.ps1 <dir>; Office writes, Ely edits, Office reads.
param([Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = 'Stop'
$Dir = (New-Item -ItemType Directory -Force $Dir).FullName
$ely = (New-Item -ItemType Directory -Force (Join-Path $Dir 'ely')).FullName
# Exact before -> after; any other value fails.
$known = @{
    'big.docx tocFields' = @('56 -> 1', 'Writer writes TOC page numbers as text')
    'big.docx body' = @('1961:333EDF4C7635 -> 1960:8E5FE0D94301', 'Writer drops empty section-break paragraphs; an untitled TOC field ends in its last entry (R15.11c)')
    'big.docx pages' = @('105:5614230D39F7 -> 105:359DB6FEE34A', 'an untitled TOC gains a line on update (R15.11c)')
    'big.xlsx Pivot values' = @('84:7A6FCE40E019 -> 84:257FC29459A5', 'Calc sums pivot data in its own order: totals differ past the 13th digit')
    'big.xlsx Pivot formulas' = @('84:F449FD4F434D -> 84:DF2CC42F1714', 'Calc sums pivot data in its own order: totals differ past the 13th digit')
    'big.pptx sections' = @('3 -> 0', 'Impress has no sections; ElyShow saves a copy')
}

function Invoke-Step([string]$What, [scriptblock]$Run) {
    $out = & $Run
    if ($LASTEXITCODE) { throw "$What failed ($LASTEXITCODE)" }
    $out
}

foreach ($make in 'make-word', 'make-excel', 'make-powerpoint') {
    Invoke-Step $make { pwsh -NoProfile -File (Join-Path $PSScriptRoot "$make.ps1") $Dir } | Out-Host
}
$built = Invoke-Step 'build' { cargo build -p ely-lok --bins --example office-roundtrip --message-format=json-render-diagnostics }
$trip = @($built | ConvertFrom-Json | Where-Object { $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'office-roundtrip' }).executable
if (-not $trip) { throw 'cargo reported no office-roundtrip executable' }
$failed = 0
$seen = @{}
foreach ($name in 'big.docx', 'big.xlsx', 'big.pptx') {
    $timing = Invoke-Step "round trip of $name" { & $trip (Join-Path $Dir $name) (Join-Path $ely $name) }
    $before, $after = foreach ($side in $Dir, $ely) {
        Invoke-Step "read-back of $side\$name" { pwsh -NoProfile -File (Join-Path $PSScriptRoot 'read-back.ps1') (Join-Path $side $name) } |
            ConvertFrom-Json -AsHashtable
    }
    "$name $timing"
    if ($before.edits -ne 0 -or $after.edits -lt 1) { "  EDIT LOST: $($before.edits) -> $($after.edits)"; $failed++ }
    foreach ($key in $before.Keys | Where-Object { $_ -ne 'edits' }) {
        if ("$($before[$key])" -eq "$($after[$key])") { continue }
        $label, $change = "$name $key", "$($before[$key]) -> $($after[$key])"
        if ($known[$label] -and $known[$label][0] -eq $change) { "  known $label`: $change ($($known[$label][1]))"; $seen[$label] = $true }
        else { "  LOST $label`: $change"; $failed++ }
    }
}
foreach ($label in $known.Keys | Where-Object { -not $seen[$_] }) { "STALE known difference: $label"; $failed++ }
if ($failed) { "$failed difference(s)"; exit 1 }
'Office reads back everything Ely saved'
