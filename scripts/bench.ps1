# Usage: bench.ps1 -Exe app.exe -File doc [-Log ely.log] [-Runs 5]
param(
    [Parameter(Mandatory)] [string] $Exe,
    [Parameter(Mandatory)] [string] $File,
    [string] $Log,
    [string] $Mark = "first tile drawn",
    [int] $Runs = 5,
    [int] $Rest = 15
)
$ErrorActionPreference = "Stop"
$name = [IO.Path]::GetFileNameWithoutExtension($Exe)
$title = [IO.Path]::GetFileNameWithoutExtension($File)

# The size of `path`, or 0 before it exists.
function Size($path) {
    if (Test-Path $path) { (Get-Item $path).Length } else { 0 }
}

# Lines `path` gained past byte `from`.
function Since($path, $from) {
    if (-not (Test-Path $path)) { return @() }
    $stream = [IO.File]::Open($path, "Open", "Read", "ReadWrite")
    try {
        $stream.Seek($from, "Begin") | Out-Null
        $text = (New-Object IO.StreamReader($stream)).ReadToEnd()
    } finally { $stream.Dispose() }
    $text -split "`n"
}

$times = @()
Get-Process $name -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds $Rest
for ($run = 0; $run -lt $Runs; $run++) {
    $from = if ($Log) { Size $Log } else { 0 }
    $start = [DateTime]::UtcNow
    $process = Start-Process $Exe -ArgumentList "`"$File`"" -PassThru
    $took = $null
    while ($null -eq $took) {
        $now = [DateTime]::UtcNow
        if (($now - $start).TotalSeconds -gt 60) { throw "run ${run}: nothing within 60 s" }
        if ($Log) {
            $line = Since $Log $from | Where-Object { $_ -match [regex]::Escape($Mark) } | Select-Object -First 1
            if ($line) {
                if ($line -notmatch '^\[(\S+Z)') { throw "run ${run}: no stamp on '$line'" }
                $at = [DateTime]::Parse($Matches[1]).ToUniversalTime()
                $took = [int]($at - $start).TotalMilliseconds
            }
        } else {
            $shown = Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like "*$title*" }
            if ($shown) { $took = [int]($now - $start).TotalMilliseconds }
        }
        if ($null -eq $took) { Start-Sleep -Milliseconds 10 }
    }
    "run ${run}: $took ms (pid $($process.Id))"
    $times += $took
    # Open a while, as a user is: it starts the next engine.
    Start-Sleep -Seconds $Rest
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
}
$sorted = $times | Sort-Object
"median $($sorted[[int][math]::Floor(($sorted.Count - 1) / 2)]) ms of $($times -join ' ')"
