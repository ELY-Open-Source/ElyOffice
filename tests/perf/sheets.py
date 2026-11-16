"""The four workbook samples, with content."""

import datetime
import zipfile
from xml.sax.saxutils import escape

S = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
R = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
P = "http://schemas.openxmlformats.org/package/2006/relationships"
T = "application/vnd.openxmlformats-officedocument.spreadsheetml"
STYLES = (f'<styleSheet xmlns="{S}"><numFmts count="2"><numFmt numFmtId="164" formatCode="#,##0.00"/>'
          '<numFmt numFmtId="165" formatCode="yyyy-mm-dd"/></numFmts><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font>'
          '<font><b/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill>'
          '<fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border/></borders>'
          '<cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="4"><xf/><xf fontId="1" applyFont="1"/>'
          '<xf numFmtId="164" applyNumberFormat="1"/><xf numFmtId="165" applyNumberFormat="1"/></cellXfs></styleSheet>')
BOLD, MONEY, DATE = 1, 2, 3

def column(n):
    name = ""
    while n:
        n, rest = divmod(n - 1, 26)
        name = chr(65 + rest) + name
    return name

def cell(ref, value, style=0):
    s = f' s="{style}"' if style else ""
    if isinstance(value, str) and value.startswith("="):
        return f'<c r="{ref}"{s}><f>{escape(value[1:])}</f></c>'
    if isinstance(value, str):
        return f'<c r="{ref}"{s} t="inlineStr"><is><t>{escape(value)}</t></is></c>'
    return f'<c r="{ref}"{s}><v>{value}</v></c>'

def rows(table):
    """Rows of (value, style) or values, from A1."""
    out = []
    for r, row in enumerate(table, 1):
        cells = []
        for c, item in enumerate(row, 1):
            value, style = item if isinstance(item, tuple) else (item, 0)
            if value is not None:
                cells.append(cell(f"{column(c)}{r}", value, style))
        out.append(f'<row r="{r}">{"".join(cells)}</row>')
    return "".join(out)

def book(path, sheets):
    """Sheets: (name, table); the parts Excel needs."""
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        kinds = "".join(f'<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="{T}.worksheet+xml"/>'
                        for i in range(1, len(sheets) + 1))
        archive.writestr("[Content_Types].xml", '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                         f'<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                         '<Default Extension="xml" ContentType="application/xml"/>'
                         f'<Override PartName="/xl/workbook.xml" ContentType="{T}.sheet.main+xml"/>'
                         f'<Override PartName="/xl/styles.xml" ContentType="{T}.styles+xml"/>{kinds}</Types>')
        archive.writestr("_rels/.rels", f'<Relationships xmlns="{P}"><Relationship Id="r1" '
                         f'Type="{R}/officeDocument" Target="xl/workbook.xml"/></Relationships>')
        links = "".join(f'<Relationship Id="s{i}" Type="{R}/worksheet" Target="worksheets/sheet{i}.xml"/>'
                        for i in range(1, len(sheets) + 1))
        archive.writestr("xl/_rels/workbook.xml.rels", f'<Relationships xmlns="{P}">{links}'
                         f'<Relationship Id="st" Type="{R}/styles" Target="styles.xml"/></Relationships>')
        names = "".join(f'<sheet name="{escape(n)}" sheetId="{i}" r:id="s{i}"/>' for i, (n, _) in enumerate(sheets, 1))
        archive.writestr("xl/workbook.xml", f'<workbook xmlns="{S}" xmlns:r="{R}"><sheets>{names}</sheets>'
                         '<calcPr fullCalcOnLoad="1"/></workbook>')
        archive.writestr("xl/styles.xml", STYLES)
        for i, (_, table) in enumerate(sheets, 1):
            archive.writestr(f"xl/worksheets/sheet{i}.xml", f'<worksheet xmlns="{S}"><sheetData>{rows(table)}</sheetData></worksheet>')

def serial(day):
    return (day - datetime.date(1899, 12, 30)).days

REGIONS = ["North", "South", "East", "West"]
PRODUCTS = ["Desk", "Chair", "Lamp", "Shelf", "Monitor", "Cable"]

def sales(n, seed=1):
    """n sales lines: date, region, product, units, price, amount."""
    start = datetime.date(2026, 1, 1)
    table = [[(h, BOLD) for h in ["Date", "Region", "Product", "Units", "Price", "Amount"]]]
    for r in range(2, n + 2):
        k = r * 7919 + seed
        table.append([(serial(start + datetime.timedelta(days=k % 365)), DATE), REGIONS[k % 4], PRODUCTS[k % 6],
                      1 + k % 40, (19.5 + k % 300, MONEY), (f"=D{r}*E{r}", MONEY)])
    return table

def dashboard(path):
    """Sales lines and a summary by region and product."""
    n = 2000
    span = lambda c: f"Sales!${c}$2:${c}${n + 1}"
    summary = [[("Region", BOLD), ("Amount", BOLD), ("Units", BOLD), ("Share", BOLD)]]
    for i, region in enumerate(REGIONS, 2):
        summary.append([region, (f'=SUMIFS({span("F")},{span("B")},A{i})', MONEY), f'=SUMIFS({span("D")},{span("B")},A{i})',
                        f"=B{i}/SUM($B$2:$B$5)"])
    summary.append([("Total", BOLD), ("=SUM(B2:B5)", MONEY), "=SUM(C2:C5)", "=SUM(D2:D5)"])
    summary.append([])
    summary.append([("Product", BOLD), ("Amount", BOLD), ("Average price", BOLD), ("Best month", BOLD)])
    for i, product in enumerate(PRODUCTS, 9):
        summary.append([product, (f'=SUMIFS({span("F")},{span("C")},A{i})', MONEY),
                        (f'=AVERAGEIFS({span("E")},{span("C")},A{i})', MONEY),
                        f'=TEXT(INDEX({span("A")},MATCH(_xlfn.MAXIFS({span("F")},{span("C")},A{i}),{span("F")},0)),"mmmm")'])
    book(path, [("Dashboard", summary), ("Sales", sales(n))])

def heavy(path):
    """Six sheets; each sums the one before."""
    n = 20000
    sheets = [("Sheet", sales(n))]
    for s in range(5):
        before = sheets[-1][0]
        table = [[("Line", BOLD), ("Carried", BOLD), ("Here", BOLD), ("Running", BOLD)]]
        for r in range(2, n + 2):
            took = f"={before}!F{r}" if s == 0 else f"={before}!D{r}"
            running = f"=B{r}+C{r}" if r == 2 else f"=D{r - 1}+B{r}+C{r}"
            table.append([r - 1, (took, MONEY), ((r * 31 + s) % 97, MONEY), (running, MONEY)])
        sheets.append((f"S_{s}", table))
    book(path, sheets)

WORDS = [("中文", "上海办公室季度报表"), ("日本語", "東京支店の売上"), ("한국어", "서울 지점 매출"),
         ("العربية", "تقرير المبيعات الشهري"), ("עברית", "דוח מכירות"), ("Русский", "Отчёт о продажах"),
         ("Ελληνικά", "Αναφορά πωλήσεων"), ("Tiếng Việt", "Báo cáo doanh thu"), ("हिन्दी", "बिक्री रिपोर्ट"),
         ("ไทย", "รายงานยอดขาย"), ("Deutsch", "Größenübersicht für Straßen"), ("Français", "Récapitulatif œuvres"),
         ("Español", "Año fiscal, señal"), ("Emoji", "📈 Q3 ✅ 🚚")]
CURRENCIES = [("CNY", "¥"), ("JPY", "¥"), ("KRW", "₩"), ("SAR", "﷼"), ("ILS", "₪"), ("RUB", "₽"), ("EUR", "€"),
              ("VND", "₫"), ("INR", "₹"), ("THB", "฿"), ("EUR", "€"), ("EUR", "€"), ("EUR", "€"), ("USD", "$")]

def international(path):
    """Scripts, right-to-left text, currencies; 1,400 lines."""
    table = [[(h, BOLD) for h in ["Language", "Title", "Currency", "Sign", "Amount", "Label", "Length"]]]
    for r in range(2, 1402):
        (lang, text), (code, sign) = WORDS[r % 14], CURRENCIES[r % 14]
        table.append([lang, f"{text} {r - 1}", code, sign, ((r * 53) % 10000 / 7, MONEY),
                      f'=D{r}&TEXT(E{r},"#,##0.00")', f"=LEN(B{r})"])
    book(path, [("Sheet", table)])

def large_dataset(path):
    """200,000 rows by eight columns, values only."""
    table = [[(h, BOLD) for h in ["Id", "Group", "A", "B", "C", "D", "Stamp", "Code"]]]
    for r in range(2, 200002):
        table.append([r - 1, f"G{r % 50}", r * 3 % 1009, r * 7 % 2003, r * 11 % 4001 / 10, r % 2,
                      (45658 + r % 730, DATE), f"K-{r * 2654435761 % 1000000:06d}"])
    book(path, [("Sheet", table)])

MAKERS = {"spytm_combo_dashboard.xlsx": dashboard, "spytm_combo_heavy.xlsx": heavy,
          "spytm_combo_international.xlsx": international, "spytm_combo_large_dataset.xlsx": large_dataset}
