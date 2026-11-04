# Usage: make-word.ps1 <dir>; Word writes big.docx there.
param([Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'patience.ps1')
$Dir = (New-Item -ItemType Directory -Force $Dir).FullName
. "$PSScriptRoot/picture.ps1"
$png = New-Picture (Join-Path $Dir 'picture.png')

# Word may still be closing the last chart's data.
function Add-Chart($sel, $doc) {
    $shapes = $doc.InlineShapes.Count
    for ($try = 1; ; $try++) {
        try { $chart = $sel.InlineShapes.AddChart2(-1, 51); break }
        catch {
            $busy = "$_" -match 'chart data grid is already open' -and $doc.InlineShapes.Count -eq $shapes
            if (-not $busy -or $try -ge 60) { throw }
            Start-Sleep -Seconds 1
        }
    }
    $chart.Chart.ChartData.Workbook.Close()
    return $chart
}

$word = New-Object -ComObject Word.Application
$word.Visible = $false
$word.DisplayAlerts = 0
try {
    $doc = $word.Documents.Add()
    $sel = $word.Selection
    $sel.Style = $doc.Styles.Item(-2)
    $sel.TypeText('Ely Office sample report')
    $sel.TypeParagraph()
    $tocAt = $sel.Range
    $sel.TypeParagraph()
    $text = ('Ely Office opens what Microsoft Office writes. 离线优先，文件在用户目录。' * 6)
    for ($s = 1; $s -le 6; $s++) {
        if ($s -gt 1) { $sel.InsertBreak(2) }
        $section = $doc.Sections.Item($doc.Sections.Count)
        if ($s -eq 3) { $section.PageSetup.Orientation = 1 }
        if ($s -eq 4) { $section.PageSetup.TextColumns.SetCount(2) }
        $sel.Style = $doc.Styles.Item(-2)
        $sel.TypeText("Chapter $s")
        $sel.TypeParagraph()
        for ($h = 1; $h -le 4; $h++) {
            $sel.Style = $doc.Styles.Item(-3)
            $sel.TypeText("Part $s.$h")
            $sel.TypeParagraph()
            $sel.Style = $doc.Styles.Item(-4)
            $sel.TypeText("Detail $s.$h")
            $sel.TypeParagraph()
            $sel.Style = $doc.Styles.Item(-1)
            for ($p = 1; $p -le 18; $p++) {
                $sel.TypeText("$s.$h.$p $text")
                $sel.TypeParagraph()
            }
        }
        # Lists, bulleted then numbered.
        foreach ($kind in 1, 2) {
            $sel.Range.ListFormat.ApplyListTemplate($word.ListGalleries.Item($kind).ListTemplates.Item(1))
            foreach ($i in 1..5) { $sel.TypeText("Item $i of list $kind"); $sel.TypeParagraph() }
            $sel.Range.ListFormat.RemoveNumbers()
        }
        $table = $doc.Tables.Add($sel.Range, 30, 6)
        $table.Borders.Enable = $true
        for ($r = 1; $r -le 30; $r++) { for ($c = 1; $c -le 6; $c++) { $table.Cell($r, $c).Range.Text = "R$r C$c" } }
        $table.Cell(1, 1).Merge($table.Cell(1, 3))
        $sel.EndKey(6) | Out-Null
        $sel.TypeParagraph()
        $doc.Footnotes.Add($sel.Range, [Type]::Missing, "Footnote of chapter $s") | Out-Null
        $sel.TypeText(' A note above.')
        $sel.TypeParagraph()
        $doc.Comments.Add($sel.Range, "Comment on chapter $s") | Out-Null
        $doc.TrackRevisions = $true
        $sel.TypeText("A tracked change in chapter $s.")
        $doc.TrackRevisions = $false
        $sel.TypeParagraph()
        $math = $sel.Range
        $math.Text = 'x=(-b±√(b^2-4ac))/2a'
        $doc.OMaths.Add($math) | Out-Null
        $doc.OMaths.Item($doc.OMaths.Count).BuildUp()
        $sel.EndKey(6) | Out-Null
        $sel.TypeParagraph()
        Add-Chart $sel $doc | Out-Null
        $sel.EndKey(6) | Out-Null
        $sel.TypeParagraph()
        $picture = $sel.InlineShapes.AddPicture($png)
        # Word sizes a picture by the display's scaling.
        $picture.Width = 300.0419
        $picture.Height = 225.0314
        $sel.TypeParagraph()
        $doc.Bookmarks.Add("chapter$s", $sel.Range) | Out-Null
        $doc.Hyperlinks.Add($sel.Range, 'https://office.elygpui.com', [Type]::Missing, [Type]::Missing, "Ely site $s") | Out-Null
        $sel.TypeParagraph()
        $sel.TypeText('See page ')
        $sel.InsertCrossReference('Bookmark', 7, "chapter$s")
        $sel.TypeParagraph()
    }
    $doc.Endnotes.Add($sel.Range, [Type]::Missing, 'The endnote') | Out-Null
    foreach ($section in $doc.Sections) {
        $section.Headers.Item(1).Range.Text = 'Ely Office sample'
        $section.Footers.Item(1).PageNumbers.Add(1) | Out-Null
    }
    $doc.TablesOfContents.Add($tocAt, $true, 1, 3) | Out-Null
    $doc.TablesOfContents.Item(1).Update()
    $out = Join-Path $Dir 'big.docx'
    $doc.SaveAs2([string]$out, 16)
    "big.docx: $($doc.ComputeStatistics(2)) pages"
    $doc.Close(0)
} finally {
    $word.Quit()
}
