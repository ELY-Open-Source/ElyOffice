# Usage: make-excel.ps1 <dir>; Excel writes source.xlsx and big.xlsx.
param([Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'patience.ps1')
$Dir = (New-Item -ItemType Directory -Force $Dir).FullName
$rows = 20000
$regions = 'North', 'South', 'East', 'West'
$products = 1..10 | ForEach-Object { "P$_" }

$excel = New-Object -ComObject Excel.Application
$excel.Visible = $false
$excel.DisplayAlerts = $false
try {
    # The linked workbook: a rate per month.
    $source = $excel.Workbooks.Add()
    # Manual: busy recalcs reject calls.
    $excel.Calculation = -4135
    $rates = $source.Worksheets.Item(1)
    $rates.Name = 'Rates'
    # Looped COM sets break later sets; write once.
    $month = New-Object 'object[,]' 12, 2
    foreach ($m in 1..12) { $month[($m - 1), 0] = $m; $month[($m - 1), 1] = 1 + $m / 100 }
    $rates.Range('A1:B12').Value2 = $month
    $sourcePath = Join-Path $Dir 'source.xlsx'
    $source.SaveAs([string]$sourcePath, 51)

    $book = $excel.Workbooks.Add()
    while ($book.Worksheets.Count -lt 5) { $book.Worksheets.Add([Type]::Missing, $book.Worksheets.Item($book.Worksheets.Count)) | Out-Null }
    $data, $calc, $charts, $pivot, $links = 1..5 | ForEach-Object { $book.Worksheets.Item($_) }
    $data.Name, $calc.Name, $charts.Name, $pivot.Name, $links.Name = 'Data', 'Calc', 'Charts', 'Pivot', 'Links'
    $values = New-Object 'object[,]' ($rows + 1), 5
    'Date', 'Region', 'Product', 'Units', 'Price' | ForEach-Object -Begin { $c = 0 } -Process { $values[0, $c++] = $_ }
    for ($r = 1; $r -le $rows; $r++) {
        $values[$r, 0] = 45658 + ($r % 365)
        $values[$r, 1] = $regions[$r % 4]
        $values[$r, 2] = $products[$r % 10]
        $values[$r, 3] = ($r * 7) % 100 + 1
        $values[$r, 4] = [math]::Round(5 + ($r % 50) / 3, 2)
    }
    $data.Range("A1:E$($rows + 1)").Value2 = $values
    $data.Range("A2:A$($rows + 1)").NumberFormat = 'yyyy-mm-dd'
    $data.Range('F1').Value2 = 'Revenue'
    $data.Range("F2:F$($rows + 1)").Formula = '=D2*E2'
    $data.ListObjects.Add(1, $data.Range("A1:F$($rows + 1)"), [Type]::Missing, 1).Name = 'Sales'
    $data.Range("D2:D$($rows + 1)").FormatConditions.AddColorScale(3) | Out-Null
    $book.Names.Add('Units', "=Data!`$D`$2:`$D`$$($rows + 1)") | Out-Null
    $book.Names.Add('Revenue', "=Data!`$F`$2:`$F`$$($rows + 1)") | Out-Null

    # Region by product: SUMIFS.
    $column = { param($i) [string][char](66 + $i) }
    foreach ($i in 0..9) { $calc.Range("$(& $column $i)1").Value2 = $products[$i] }
    foreach ($j in 0..3) {
        $calc.Range("A$($j + 2)").Value2 = $regions[$j]
        foreach ($i in 0..9) {
            $calc.Range("$(& $column $i)$($j + 2)").Formula = "=SUMIFS(Revenue,Data!`$B`$2:`$B`$$($rows + 1),`$A$($j + 2),Data!`$C`$2:`$C`$$($rows + 1),$(& $column $i)`$1)"
        }
    }
    $calc.Range('A8').Value2 = 'Lookups'
    $calc.Range('B8').Formula = "=VLOOKUP(100,Data!`$A`$1:`$F`$$($rows + 1),6,FALSE)"
    $calc.Range('B9').Formula = "=INDEX(Data!`$F`$2:`$F`$$($rows + 1),MATCH(MAX(Units),Units,0))"
    $calc.Range('B10').Formula2 = "=XLOOKUP(""P7"",Data!`$C`$2:`$C`$$($rows + 1),Revenue,0)"
    $calc.Range('B11').Formula = '=IFERROR(1/0,"none")'
    $calc.Range('B12').Formula = '=TEXT(SUM(Revenue),"#,##0.00")'
    $calc.Range('D8').Formula2 = "=SORT(UNIQUE(Data!`$B`$2:`$B`$$($rows + 1)))"
    $calc.Range('F8').Formula2 = "=TAKE(FILTER(Data!`$F`$2:`$F`$$($rows + 1),Data!`$D`$2:`$D`$$($rows + 1)>98),5)"
    $calc.Range('H8').Validation.Add(3, 1, 1, ($regions -join ','))
    $calc.Range('H8').Value2 = 'East'
    $calc.Range('H9').Formula = "=SUMIFS(Revenue,Data!`$B`$2:`$B`$$($rows + 1),H8)"

    # Linked rates; the link keeps its cached values.
    foreach ($m in 1..12) { $links.Range("A$m").Formula = "='$Dir\[source.xlsx]Rates'!`$B`$$m" }
    $links.Range('B1').Formula = '=SUM(A1:A12)'

    $add = { param($type, $range, $left, $top)
        $shape = $charts.Shapes.AddChart2(-1, $type, $left, $top, 420, 260)
        $shape.Chart.SetSourceData($range)
        $shape.Chart }
    & $add 51 $calc.Range('A1:K5') 10 10 | Out-Null
    & $add 4 $calc.Range('A1:K3') 450 10 | Out-Null
    & $add 5 $calc.Range('A1:B5') 10 290 | Out-Null
    & $add -4169 $data.Range('D1:F200') 450 290 | Out-Null
    $combo = & $add 51 $calc.Range('A1:C5') 10 570
    $combo.SeriesCollection(2).ChartType = 4
    $combo.SeriesCollection(2).AxisGroup = 2

    $cache = $book.PivotCaches().Create(1, "Data!R1C1:R$($rows + 1)C6")
    $table = $cache.CreatePivotTable("Pivot!R3C1", 'Sales pivot')
    $table.PivotFields('Region').Orientation = 1
    $table.PivotFields('Product').Orientation = 2
    $table.AddDataField($table.PivotFields('Revenue')) | Out-Null
    $data.Activate()
    $excel.ActiveWindow.FreezePanes = $false
    $excel.ActiveWindow.SplitRow = 1
    $excel.ActiveWindow.FreezePanes = $true
    $excel.Calculation = -4105
    $excel.CalculateFull()
    $out = Join-Path $Dir 'big.xlsx'
    $book.SaveAs([string]$out, 51)
    "big.xlsx: $($book.Worksheets.Count) sheets, $rows rows"
    $book.Close($false)
    $source.Close($false)
} finally {
    $excel.Quit()
}
