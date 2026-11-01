# A 400 by 300 striped PNG for the samples.
function New-Picture([string]$Path) {
    Add-Type -AssemblyName System.Drawing
    $bitmap = New-Object System.Drawing.Bitmap 400, 300
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    foreach ($i in 0..9) {
        $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 25 * $i, 120, 255 - 25 * $i))
        $graphics.FillRectangle($brush, 40 * $i, 0, 40, 300)
    }
    $graphics.Dispose()
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $Path
}
