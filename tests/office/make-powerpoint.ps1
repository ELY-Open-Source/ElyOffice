# Usage: make-powerpoint.ps1 <dir>; PowerPoint writes big.pptx there.
param([Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'patience.ps1')
$Dir = (New-Item -ItemType Directory -Force $Dir).FullName
. "$PSScriptRoot/picture.ps1"
$png = New-Picture (Join-Path $Dir 'picture.png')
$clip = (Resolve-Path (Join-Path $PSScriptRoot '../fixtures/media/clip.mp4')).Path
$slides = 60

$ppt = New-Object -ComObject PowerPoint.Application
try {
    # Charts need a window; PowerPoint cannot hide.
    $pres = $ppt.Presentations.Add(-1)
    $ppt.WindowState = 2
    $layouts = $pres.SlideMaster.CustomLayouts
    $text = 'Ely Office opens what Microsoft Office writes. 离线优先，文件在用户目录。'
    for ($n = 1; $n -le $slides; $n++) {
        $kind = $n % 6
        $layout = if ($n -eq 1) { $layouts.Item(1) } elseif ($kind -eq 0) { $layouts.Item(7) } else { $layouts.Item(2) }
        $slide = $pres.Slides.AddSlide($n, $layout)
        if ($slide.Shapes.HasTitle) { $slide.Shapes.Title.TextFrame.TextRange.Text = "Slide $n" }
        $body = if ($slide.Shapes.Placeholders.Count -ge 2) { $slide.Shapes.Placeholders.Item(2) }
        switch ($kind) {
            1 { $body.TextFrame.TextRange.Text = (1..6 | ForEach-Object { "Point $_ $text" }) -join "`r" }
            2 {
                $body.Delete()
                $table = $slide.Shapes.AddTable(8, 5, 60, 140, 840, 340).Table
                for ($r = 1; $r -le 8; $r++) { for ($c = 1; $c -le 5; $c++) { $table.Cell($r, $c).Shape.TextFrame.TextRange.Text = "R$r C$c" } }
                $table.Cell(1, 1).Merge($table.Cell(1, 2))
            }
            3 {
                $body.Delete()
                $chart = $slide.Shapes.AddChart2(-1, (51, 4, 5)[$n % 3], 60, 140, 840, 360).Chart
                $chart.ChartData.Workbook.Close()
            }
            4 {
                $body.Delete()
                $slide.Shapes.AddSmartArt($ppt.SmartArtLayouts.Item(1), 60, 140, 840, 360) | Out-Null
            }
            5 {
                $body.TextFrame.TextRange.Text = 'Site and picture'
                $body.TextFrame.TextRange.ActionSettings.Item(1).Hyperlink.Address = 'https://office.elygpui.com'
                $slide.Shapes.AddPicture($png, 0, -1, 520, 200, 400, 300) | Out-Null
            }
            0 {
                $a = $slide.Shapes.AddShape(1, 100, 120, 300, 160)
                $b = $slide.Shapes.AddShape(9, 450, 120, 300, 160)
                $a.TextFrame.TextRange.Text = "Group $n"
                $slide.Shapes.Range([object[]]@($a.Name, $b.Name)).Group() | Out-Null
                if ($n -eq 12) { $slide.Shapes.AddMediaObject2($clip, 0, -1, 100, 320, 320, 180) | Out-Null }
            }
        }
        $slide.SlideShowTransition.EntryEffect = 3849
        $slide.TimeLine.MainSequence.AddEffect($slide.Shapes.Item($slide.Shapes.Count), 10) | Out-Null
        $slide.NotesPage.Shapes.Placeholders.Item(2).TextFrame.TextRange.Text = "Notes for slide $n"
    }
    $pres.SectionProperties.AddBeforeSlide(1, 'Opening') | Out-Null
    $pres.SectionProperties.AddBeforeSlide(21, 'Middle') | Out-Null
    $pres.SectionProperties.AddBeforeSlide(41, 'Close') | Out-Null
    $out = Join-Path $Dir 'big.pptx'
    $pres.SaveAs([string]$out, 24)
    "big.pptx: $($pres.Slides.Count) slides, $($pres.SectionProperties.Count) sections"
    $pres.Close()
} finally {
    $ppt.Quit()
}
