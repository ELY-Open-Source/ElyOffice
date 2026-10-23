//! Windows: the system print dialog; Windows' PDF renderer draws.

use std::path::Path;

use anyhow::{Context as _, anyhow, bail};
use windows::{
    Data::Pdf::{PdfDocument, PdfPage, PdfPageRenderOptions},
    Foundation::{Rect, Size},
    Graphics::Imaging::BitmapEncoder,
    Storage::{
        StorageFile,
        Streams::{DataReader, InMemoryRandomAccessStream},
    },
    Win32::{
        Foundation::ERROR_CANCELLED,
        Graphics::Gdi::{
            BITMAPINFO, CreateDCW, DIB_RGB_COLORS, DeleteDC, GM_ADVANCED, GetDeviceCaps, HDC, LOGPIXELSX, LOGPIXELSY,
            PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH, SRCCOPY, SetGraphicsMode, SetWorldTransform,
            StretchDIBits, XFORM,
        },
        Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage},
        System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        UI::{
            Controls::Dialogs::{
                PD_NOCURRENTPAGE, PD_NOSELECTION, PD_PAGENUMS, PD_PRINTTOFILE, PD_RESULT_PRINT, PD_USEDEVMODECOPIESANDCOLLATE,
                PRINTDLGEX_FLAGS, PRINTDLGEXW, PRINTPAGERANGE, PrintDlgExW, START_PAGE_GENERAL,
            },
            Input::KeyboardAndMouse::GetActiveWindow,
        },
    },
    core::{HSTRING, PCWSTR},
};

mod printer;

use printer::{Devmode, default_printer, device, free, global, names, settings};

use crate::settings::Sides;

/// The printer tests write their job to a file with.
const TO_FILE: &str = "Microsoft Print to PDF";
/// Bytes one rendered band may take.
const BAND: usize = 32 << 20;
/// The probe's width, in pixels asked.
const PROBE: u32 = 200;

/// Shows the print dialog, then prints; false when cancelled.
pub async fn print_pdf(pdf: &Path, sides: Sides) -> anyhow::Result<bool> {
    let pdf = std::path::absolute(pdf)?;
    let sizes = {
        let pdf = pdf.clone();
        super::off(move || winrt(|| sizes(&open(&pdf)?))).await?
    };
    let paper = super::paper_for(&sizes).ok_or_else(|| anyhow!("{} has no pages", pdf.display()))?;
    log::info!("print: paper {}x{} points", paper.0, paper.1);
    let every = (0..sizes.len() as u32).collect();
    // Tests read the real job: written by the file printer.
    let job = match crate::env::value("ELY_PRINT_TO") {
        Some(to) => {
            let printer = wide(TO_FILE);
            let settings = settings(&printer, paper, sides)?;
            Job { printer, settings, pages: every, output: Some(wide(&to)) }
        }
        None => match ask(every, paper, sides)? {
            Some(job) => job,
            None => {
                log::info!("print: the dialog was cancelled");
                return Ok(false);
            }
        },
    };
    let printed = super::off(move || winrt(|| job.print(&pdf))).await?;
    log::info!("print: the job ended, printed {printed}");
    Ok(printed)
}

/// What the dialog chose.
struct Job {
    printer: Vec<u16>,
    settings: Devmode,
    pages: Vec<u32>,
    output: Option<Vec<u16>>,
}

impl Job {
    /// Draws the chosen pages; false when its save prompt cancels.
    fn print(self, pdf: &Path) -> anyhow::Result<bool> {
        let document = open(pdf)?;
        let dc = unsafe { CreateDCW(PCWSTR::null(), PCWSTR(self.printer.as_ptr()), PCWSTR::null(), Some(self.settings.as_ptr())) };
        if dc.is_invalid() {
            bail!("the printer cannot be opened: {}", std::io::Error::last_os_error());
        }
        let printed = self.draw(&document, dc);
        if !unsafe { DeleteDC(dc) }.as_bool() {
            log::warn!("print: the printer's context stays open");
        }
        printed
    }

    fn draw(&self, document: &PdfDocument, dc: HDC) -> anyhow::Result<bool> {
        // The driver may have swapped our paper; nothing is cut.
        let paper = Paper::of(dc);
        for &n in &self.pages {
            let size = document.GetPage(n)?.Size()?;
            if paper.fit(size).is_none() {
                let (width, height) = paper.millimetres();
                let mm = |dips: f32| f64::from(dips) * 25.4 / 96.0;
                bail!(
                    "page {} is {:.0} × {:.0} mm; the printer's paper is {width:.0} × {height:.0} mm",
                    n + 1,
                    mm(size.Width),
                    mm(size.Height)
                );
            }
        }
        let name = wide("Ely Office");
        let info = DOCINFOW {
            cbSize: size_of::<DOCINFOW>() as i32,
            lpszDocName: PCWSTR(name.as_ptr()),
            lpszOutput: self.output.as_ref().map_or(PCWSTR::null(), |output| PCWSTR(output.as_ptr())),
            ..Default::default()
        };
        if unsafe { StartDocW(dc, &info) } <= 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_CANCELLED.0 as i32) {
                return Ok(false);
            }
            bail!("the printer refused the job: {error}");
        }
        let drawn = scale(document).and_then(|scale| self.pages.iter().try_for_each(|&n| page(document, n, dc, &paper, scale)));
        if let Err(error) = drawn {
            unsafe { AbortDoc(dc) };
            return Err(error);
        }
        if unsafe { EndDoc(dc) } <= 0 {
            bail!("the printer did not finish the job: {}", std::io::Error::last_os_error());
        }
        Ok(true)
    }
}

/// Shows the dialog over the active window; None when cancelled.
fn ask(every: Vec<u32>, paper: (f64, f64), sides: Sides) -> anyhow::Result<Option<Job>> {
    let owner = unsafe { GetActiveWindow() };
    if owner.is_invalid() {
        bail!("no window is active to own the print dialog");
    }
    let printer = default_printer()?;
    let settings = settings(&printer, paper, sides)?;
    let mut ranges = [PRINTPAGERANGE::default(); 32];
    let mut dialog = PRINTDLGEXW {
        lStructSize: size_of::<PRINTDLGEXW>() as u32,
        hwndOwner: owner,
        hDevMode: global(settings.bytes())?,
        hDevNames: global(&names(&printer))?,
        Flags: PD_NOSELECTION | PD_NOCURRENTPAGE | PD_USEDEVMODECOPIESANDCOLLATE,
        nMaxPageRanges: ranges.len() as u32,
        lpPageRanges: ranges.as_mut_ptr(),
        nMinPage: 1,
        nMaxPage: every.len() as u32,
        nCopies: 1,
        nStartPage: START_PAGE_GENERAL,
        ..Default::default()
    };
    let shown = unsafe { PrintDlgExW(&mut dialog) };
    let chosen = shown.map_err(anyhow::Error::from).and_then(|()| {
        if dialog.dwResultAction != PD_RESULT_PRINT {
            return Ok(None);
        }
        let (pages, output) = chosen(dialog.Flags, &ranges[..dialog.nPageRanges as usize], every.len() as u32);
        Ok(Some(Job { printer: device(dialog.hDevNames)?, settings: Devmode::copied(dialog.hDevMode)?, pages, output }))
    });
    // The dialog may replace either block; we free both.
    free(dialog.hDevMode)?;
    free(dialog.hDevNames)?;
    chosen
}

/// The pages and output the dialog's answer chose.
fn chosen(flags: PRINTDLGEX_FLAGS, ranges: &[PRINTPAGERANGE], count: u32) -> (Vec<u32>, Option<Vec<u16>>) {
    let pages = match flags.contains(PD_PAGENUMS) {
        true => ranges.iter().flat_map(|range| range.nFromPage.max(1) - 1..range.nToPage.min(count)).collect(),
        false => (0..count).collect(),
    };
    // "FILE:" makes the spooler ask for the file's name.
    let output = flags.contains(PD_PRINTTOFILE).then(|| wide("FILE:"));
    (pages, output)
}

/// The printer's paper, in its device pixels.
struct Paper {
    dpi: (i32, i32),
    size: (i32, i32),
    offset: (i32, i32),
}

impl Paper {
    fn of(dc: HDC) -> Self {
        let caps = |index| unsafe { GetDeviceCaps(Some(dc), index) };
        Self {
            dpi: (caps(LOGPIXELSX), caps(LOGPIXELSY)),
            size: (caps(PHYSICALWIDTH), caps(PHYSICALHEIGHT)),
            offset: (caps(PHYSICALOFFSETX), caps(PHYSICALOFFSETY)),
        }
    }

    /// Pixels, and whether it turns; None if too large.
    fn fit(&self, size: Size) -> Option<((i32, i32), bool)> {
        let ((dpi_x, dpi_y), (paper_x, paper_y)) = (self.dpi, self.size);
        let pixels = |dips: f32, dpi: i32| (f64::from(dips) * f64::from(dpi) / 96.0).round() as i32;
        // A page fits upright or turned, within rounding.
        let fits = |pixels: i32, paper: i32, dpi: i32| f64::from(pixels - paper) <= f64::from(dpi) * super::ROUNDING / 72.0;
        let (across, down) = (pixels(size.Width, dpi_x), pixels(size.Height, dpi_y));
        if fits(across, paper_x, dpi_x) && fits(down, paper_y, dpi_y) {
            return Some(((across, down), false));
        }
        let (across, down) = (pixels(size.Width, dpi_y), pixels(size.Height, dpi_x));
        (fits(across, paper_y, dpi_y) && fits(down, paper_x, dpi_x)).then_some(((across, down), true))
    }

    fn millimetres(&self) -> (f64, f64) {
        let mm = |pixels: i32, dpi: i32| f64::from(pixels) * 25.4 / f64::from(dpi);
        (mm(self.size.0, self.dpi.0), mm(self.size.1, self.dpi.1))
    }
}

/// Pixels given per pixel asked: the renderer counts DIPs.
fn scale(document: &PdfDocument) -> anyhow::Result<f64> {
    let options = PdfPageRenderOptions::new()?;
    options.SetDestinationWidth(PROBE)?;
    options.SetDestinationHeight(PROBE)?;
    let (across, _) = bitmap(&rendered(&document.GetPage(0)?, &options)?)?.size;
    log::info!("print: the renderer gives {across} pixels per {PROBE} asked");
    Ok(f64::from(across) / f64::from(PROBE))
}

/// Draws page `n` at its size from the paper's corner.
fn page(document: &PdfDocument, n: u32, dc: HDC, paper: &Paper, scale: f64) -> anyhow::Result<()> {
    let page = document.GetPage(n)?;
    let size = page.Size()?;
    let ((width, height), turned) = paper.fit(size).with_context(|| format!("page {} does not fit the paper", n + 1))?;
    let (offset_x, offset_y) = paper.offset;
    if unsafe { StartPage(dc) } <= 0 {
        bail!("the printer refused page {}: {}", n + 1, std::io::Error::last_os_error());
    }
    // Device pixels start at the printable area, not the paper.
    let place = match turned {
        // A turned page's top runs up its left edge.
        true => XFORM { eM11: 0.0, eM12: -1.0, eM21: 1.0, eM22: 0.0, eDx: -offset_x as f32, eDy: (width - offset_y) as f32 },
        false => XFORM { eM11: 1.0, eM12: 0.0, eM21: 0.0, eM22: 1.0, eDx: -offset_x as f32, eDy: -offset_y as f32 },
    };
    if unsafe { SetGraphicsMode(dc, GM_ADVANCED) } == 0 || !unsafe { SetWorldTransform(dc, &place) }.as_bool() {
        bail!("the printer cannot place page {}", n + 1);
    }
    let rows = (BAND / (width.max(1) as usize * 4)).max(1) as i32;
    let mut top = 0;
    while top < height {
        let band = rows.min(height - top);
        let bmp = render(&page, size, (width, height), (top, band), scale)?;
        draw(dc, &bmp, top, width, band)?;
        top += band;
    }
    if unsafe { EndPage(dc) } <= 0 {
        bail!("the printer did not take page {}: {}", n + 1, std::io::Error::last_os_error());
    }
    Ok(())
}

/// The page's band from row `top`, as a BMP file.
fn render(page: &PdfPage, size: Size, (width, height): (i32, i32), (top, rows): (i32, i32), scale: f64) -> anyhow::Result<Vec<u8>> {
    let options = PdfPageRenderOptions::new()?;
    let row = f64::from(size.Height) / f64::from(height);
    options.SetSourceRect(Rect {
        X: 0.0,
        Y: (f64::from(top) * row) as f32,
        Width: size.Width,
        Height: (f64::from(rows) * row) as f32,
    })?;
    let asked = |pixels: i32| ((f64::from(pixels) / scale).round() as u32).max(1);
    options.SetDestinationWidth(asked(width))?;
    options.SetDestinationHeight(asked(rows))?;
    rendered(page, &options)
}

/// The page as `options` asks, as a BMP file.
fn rendered(page: &PdfPage, options: &PdfPageRenderOptions) -> anyhow::Result<Vec<u8>> {
    options.SetBitmapEncoderId(BitmapEncoder::BmpEncoderId()?)?;
    let stream = InMemoryRandomAccessStream::new()?;
    page.RenderWithOptionsToStreamAsync(&stream, options)?.join()?;
    let length = u32::try_from(stream.Size()?)?;
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    reader.LoadAsync(length)?.join()?;
    let mut bytes = vec![0; length as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

/// A BMP file's pixel size and where its pixels start.
struct Bitmap {
    size: (i32, i32),
    start: usize,
}

fn bitmap(bmp: &[u8]) -> anyhow::Result<Bitmap> {
    let word = |at: usize| bmp.get(at..at + 4).map(|bytes| u32::from_le_bytes(bytes.try_into().expect("four bytes")));
    let (Some(start), Some(across), Some(down)) = (word(10), word(18), word(22)) else {
        bail!("the renderer gave no bitmap");
    };
    if !bmp.starts_with(b"BM") || start as usize >= bmp.len() {
        bail!("the renderer gave a broken bitmap");
    }
    // Rows run up the file when its height is positive.
    Ok(Bitmap { size: (across as i32, (down as i32).abs()), start: start as usize })
}

/// Stretches one BMP band over `width` by `rows` at `top`.
fn draw(dc: HDC, bmp: &[u8], top: i32, width: i32, rows: i32) -> anyhow::Result<()> {
    let Bitmap { size: (across, down), start } = bitmap(bmp)?;
    // Asking in DIPs rounds; a few pixels off is that.
    if (across - width).abs() > 4 || (down - rows).abs() > 4 {
        bail!("the renderer gave a {across}x{down} bitmap for {width}x{rows}");
    }
    let info = bmp[14..].as_ptr().cast::<BITMAPINFO>();
    let bits = bmp[start..].as_ptr().cast();
    let copied = unsafe { StretchDIBits(dc, 0, top, width, rows, 0, 0, across, down, Some(bits), info, DIB_RGB_COLORS, SRCCOPY) };
    if copied <= 0 {
        bail!("the printer took none of the page: {}", std::io::Error::last_os_error());
    }
    Ok(())
}

/// Opens `pdf` with Windows' PDF renderer.
fn open(pdf: &Path) -> anyhow::Result<PdfDocument> {
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(pdf))?
        .join()
        .with_context(|| format!("{} cannot be opened", pdf.display()))?;
    PdfDocument::LoadFromFileAsync(&file)?.join().with_context(|| format!("{} is no readable PDF", pdf.display()))
}

/// Each page's size in points, as it shows.
fn sizes(document: &PdfDocument) -> anyhow::Result<Vec<(f64, f64)>> {
    let points = |dips: f32| f64::from(dips) * 72.0 / 96.0;
    (0..document.PageCount()?)
        .map(|n| {
            let size = document.GetPage(n)?.Size()?;
            Ok((points(size.Width), points(size.Height)))
        })
        .collect()
}

/// Runs `work` with the thread joined to the Windows Runtime.
fn winrt<T>(work: impl FnOnce() -> anyhow::Result<T>) -> anyhow::Result<T> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }?;
    let result = work();
    unsafe { RoUninitialize() };
    result
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::Controls::Dialogs::{PD_PAGENUMS, PD_PRINTTOFILE, PRINTDLGEX_FLAGS, PRINTPAGERANGE};

    use super::{chosen, wide};

    #[test]
    fn the_dialog_answer_picks_pages_and_output() {
        let ranges = [PRINTPAGERANGE { nFromPage: 2, nToPage: 3 }, PRINTPAGERANGE { nFromPage: 5, nToPage: 9 }];
        assert_eq!(chosen(PRINTDLGEX_FLAGS(0), &ranges, 3), (vec![0, 1, 2], None));
        assert_eq!(chosen(PD_PAGENUMS, &ranges, 6), (vec![1, 2, 4, 5], None));
        // Print to file: the spooler asks where.
        assert_eq!(chosen(PD_PAGENUMS | PD_PRINTTOFILE, &ranges[..1], 6), (vec![1, 2], Some(wide("FILE:"))));
    }
}
