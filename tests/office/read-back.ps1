# Usage: read-back.ps1 <file>; Office reads counts and content digests.
param([Parameter(Mandatory)][string]$File)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'patience.ps1')
$File = (Resolve-Path $File).Path
$mark = 'Ely edit '

function Get-Digest($Items) {
    $text = (@($Items) | ForEach-Object { "$_" }) -join "`n"
    $hash = [Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($text))
    "$(@($Items).Count):$([Convert]::ToHexString($hash).Substring(0, 12))"
}

# A field's first argument; URLs as the URI.
function Get-Target([string]$Code) {
    $words = @($Code.Trim() -split '\s+')
    $target = if ($words.Count -gt 1) { $words[1].Trim('"') } else { '' }
    $uri = $null
    if ([Uri]::TryCreate($target, 'Absolute', [ref]$uri)) { $uri.AbsoluteUri } else { $target }
}

# A value with its type; dates stay dates.
function Get-Typed($Value) {
    if ($null -eq $Value) { '' } else { "$($Value.GetType().Name):$Value" }
}

function Get-Series($Chart) {
    # Undrawn charts may read empty categories.
    $drawn = [IO.Path]::GetTempFileName()
    [void]$Chart.Export($drawn, 'PNG')
    Remove-Item $drawn
    foreach ($s in $Chart.SeriesCollection()) {
        "$($s.ChartType) $($s.AxisGroup) $($s.Formula) | $(@($s.XValues) -join ',') | $(@($s.Values) -join ',')"
    }
}

# Whether each chart's first series follows its cells.
function Get-Live($Book) {
    foreach ($sheet in $Book.Worksheets) {
        foreach ($c in $sheet.ChartObjects()) {
            $s = $c.Chart.SeriesCollection(1)
            $ref = ($s.Formula -split ',')[2]
            if ($ref -notmatch '^''?([^''!]+)''?!(\$?[A-Z]+\$?\d+(:\$?[A-Z]+\$?\d+)?)$') { "$($c.Name) unlinked"; continue }
            $cell = $Book.Worksheets.Item($Matches[1]).Range($Matches[2]).Cells.Item(1)
            $before = @($s.Values)[0]
            $cell.Value2 = [double]$cell.Value2 + 1000
            "$($c.Name) $(@($s.Values)[0] -ne $before)"
        }
    }
}

# A range's text, each field a {field} mark.
function Get-Literal($Range) {
    $at = $Range.Start
    $part = $Range.Duplicate
    foreach ($f in $Range.Fields) {
        # Field chars: begin before Code, end after Result.
        $part.SetRange($at, $f.Code.Start - 1)
        $part.Text + '{field}'
        $at = $f.Result.End + 1
    }
    $part.SetRange($at, $Range.End)
    $part.Text
}

# Text and fields, frames or text boxes alike.
function Get-Margin($Part) {
    if (-not $Part.Exists) { return }
    # Part.Shapes lists every margin's; ShapeRange this one's.
    $own = $Part.Range
    $boxes = foreach ($s in $own.ShapeRange) { if ($s.TextFrame.HasText) { $s.TextFrame.TextRange } }
    $ranges = @($own) + @($boxes)
    $text = foreach ($r in $ranges) { Get-Literal $r }
    $codes = foreach ($r in $ranges) { foreach ($f in $r.Fields) { "$($f.Type) $(Get-Target $f.Code.Text)" } }
    "$((($text -join ' ') -replace '[\s\a]+', ' ').Trim()) | $(@($codes | Sort-Object) -join ',')"
}

function Read-Word([string]$Path) {
    $word = New-Object -ComObject Word.Application
    $word.DisplayAlerts = 0
    try {
        $doc = $word.Documents.Open($Path, $false, $true)
        $toc = $doc.TablesOfContents.Item(1).Range
        $tocFields = @($doc.Fields | Where-Object { $_.Code.Start -ge $toc.Start -and $_.Code.End -le $toc.End }).Count
        $starts = (@(foreach ($s in $doc.Sections) { $s.Range.Characters.First.Information(3) }) -join ',')
        $links = Get-Digest @(foreach ($l in $doc.Hyperlinks) { "$($l.Address) $($l.TextToDisplay)" })
        # Page numbers by Word's layout, both sides.
        $doc.Fields.Update() | Out-Null
        $doc.Repaginate()
        # What each page starts with, after the update.
        $pages = foreach ($n in 1..$doc.ComputeStatistics(2)) {
            $at = $doc.GoTo(1, 1, $n)
            $para = $at.Paragraphs.Item(1).Range
            "$n $($at.Start - $para.Start) $(($para.Text -replace $mark, '').Trim())"
        }
        $toc = $doc.TablesOfContents.Item(1).Range
        $outside = @($doc.Fields | Where-Object { $_.Code.Start -lt $toc.Start -or $_.Code.End -gt $toc.End })
        $margins = foreach ($s in $doc.Sections) {
            foreach ($kind in 1, 2, 3) { "$($s.Index) header $kind $(Get-Margin $s.Headers.Item($kind))"; "$($s.Index) footer $kind $(Get-Margin $s.Footers.Item($kind))" }
        }
        [ordered]@{
            edits = ($doc.Content.Text -split $mark).Count - 1
            body = Get-Digest ($doc.Content.Text -replace $mark, '' -split "`r")
            tables = Get-Digest @(foreach ($t in $doc.Tables) { "$($t.Rows.Count)x$($t.Columns.Count) $($t.Range.Text -replace $mark, '')" })
            footnotes = Get-Digest @(foreach ($n in $doc.Footnotes) { $n.Range.Text })
            endnotes = Get-Digest @(foreach ($n in $doc.Endnotes) { $n.Range.Text })
            comments = Get-Digest @(foreach ($c in $doc.Comments) { "$($c.Scope.Text) $($c.Range.Text)" })
            revisions = Get-Digest @(foreach ($r in $doc.Revisions) { "$($r.Type) $($r.Range.Text)" })
            equations = Get-Digest @(foreach ($m in $doc.OMaths) { $m.Range.Text })
            inline = Get-Digest @(foreach ($s in $doc.InlineShapes) { "$($s.Type) $([int]$s.Width)x$([int]$s.Height)" })
            bookmarks = Get-Digest @(foreach ($b in $doc.Bookmarks) { $b.Name })
            margins = Get-Digest @($margins)
            charts = Get-Digest @(foreach ($s in $doc.InlineShapes) { if ($s.HasChart -eq -1) { Get-Series $s.Chart } })
            fields = Get-Digest @(foreach ($f in $outside) { "$($f.Type) $(Get-Target $f.Code.Text)" })
            pages = Get-Digest @($pages)
            tocFields = $tocFields; starts = $starts; links = $links
        }
        $doc.Close(0)
    } finally { $word.Quit() }
}

function Read-Excel([string]$Path) {
    $excel = New-Object -ComObject Excel.Application
    $excel.DisplayAlerts = $false
    try {
        $book = $excel.Workbooks.Open($Path, 0, $true)
        $edit = $book.Worksheets.Item('Calc').Range('A12')
        $edits = [int]($edit.Value2 -eq $mark.Trim())
        $edit.ClearContents() | Out-Null
        $excel.CalculateFull()
        $found = [ordered]@{ edits = $edits; links = @($book.LinkSources(1) | Where-Object { $_ }).Count
            names = Get-Digest @(foreach ($n in $book.Names) { "$($n.Name) $($n.RefersTo)" }) }
        foreach ($sheet in $book.Worksheets) {
            $used = $sheet.UsedRange
            $found["$($sheet.Name) used"] = $used.Address()
            $found["$($sheet.Name) values"] = Get-Digest @(foreach ($v in $used.Value()) { Get-Typed $v })
            $found["$($sheet.Name) formulas"] = Get-Digest @(foreach ($v in $used.Formula2) { $v })
            $found["$($sheet.Name) charts"] = Get-Digest @(foreach ($c in $sheet.ChartObjects()) { Get-Series $c.Chart })
            $found["$($sheet.Name) pivots"] = Get-Digest @(foreach ($p in $sheet.PivotTables()) { "$($p.Name) $($p.TableRange2.Address())" })
            $found["$($sheet.Name) tables"] = Get-Digest @(foreach ($t in $sheet.ListObjects) { "$($t.Name) $($t.Range.Address())" })
        }
        $found.live = (@(Get-Live $book) -join '; ')
        $found
        $book.Close($false)
    } finally { $excel.Quit() }
}

function Get-ShapeText($Shape) {
    if ($Shape.Type -eq 6) { foreach ($item in $Shape.GroupItems) { Get-ShapeText $item } }
    if ($Shape.HasTable) { foreach ($cell in $Shape.Table.Rows) { foreach ($c in $cell.Cells) { $c.Shape.TextFrame.TextRange.Text } } }
    if ($Shape.HasSmartArt) { foreach ($node in $Shape.SmartArt.AllNodes) { $node.TextFrame2.TextRange.Text } }
    if ($Shape.HasTextFrame) { $Shape.TextFrame.TextRange.Text }
}

function Read-PowerPoint([string]$Path) {
    $ppt = New-Object -ComObject PowerPoint.Application
    try {
        $pres = $ppt.Presentations.Open($Path, -1, 0, 0)
        $text = @(); $notes = @(); $kinds = @(); $motion = @()
        foreach ($slide in $pres.Slides) {
            foreach ($shape in $slide.Shapes) {
                $text += @(Get-ShapeText $shape)
                $kinds += "$($slide.SlideIndex) $($shape.Type) $($shape.HasChart) $($shape.HasSmartArt) $([int]$shape.Left),$([int]$shape.Top)"
                if ($shape.HasChart) { $kinds += @(Get-Series $shape.Chart) }
            }
            $notes += $slide.NotesPage.Shapes.Placeholders.Item(2).TextFrame.TextRange.Text
            $motion += "$($slide.SlideShowTransition.EntryEffect) " + (@(foreach ($e in $slide.TimeLine.MainSequence) { "$($e.EffectType)/$($e.Timing.Duration)" }) -join ',')
        }
        [ordered]@{
            edits = @($text | Where-Object { $_ -like "*$mark*" }).Count
            slides = $pres.Slides.Count; sections = $pres.SectionProperties.Count
            text = Get-Digest ($text | ForEach-Object { $_ -replace $mark, '' })
            notes = Get-Digest $notes; shapes = Get-Digest $kinds; motion = Get-Digest $motion
        }
        $pres.Close()
    } finally { $ppt.Quit() }
}

$found = switch ([IO.Path]::GetExtension($File)) {
    '.docx' { Read-Word $File }
    '.xlsx' { Read-Excel $File }
    '.pptx' { Read-PowerPoint $File }
    default { throw "not an Office file: $File" }
}
$found | ConvertTo-Json -Compress
