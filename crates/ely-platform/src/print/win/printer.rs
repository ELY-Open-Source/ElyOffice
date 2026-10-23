//! The printer's settings and the global blocks the dialog trades.

use anyhow::{Context as _, bail};
use windows::{
    Win32::{
        Foundation::{GlobalFree, HGLOBAL, POINT},
        Graphics::{
            Gdi::{
                DEVMODE_DUPLEX, DEVMODEW, DM_DUPLEX, DM_IN_BUFFER, DM_ORIENTATION, DM_OUT_BUFFER, DM_PAPERLENGTH, DM_PAPERSIZE,
                DM_PAPERWIDTH, DMDUP_HORIZONTAL, DMDUP_SIMPLEX, DMDUP_VERTICAL, DMORIENT_LANDSCAPE, DMORIENT_PORTRAIT, DMPAPER_USER,
            },
            Printing::{ClosePrinter, DocumentPropertiesW, GetDefaultPrinterW, OpenPrinterW, PRINTER_HANDLE},
        },
        Storage::Xps::{DC_PAPERS, DC_PAPERSIZE, DeviceCapabilitiesW},
        System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock},
        UI::Controls::Dialogs::DEVNAMES,
    },
    core::{PCWSTR, PWSTR},
};

use super::wide;
use crate::settings::Sides;

/// `printer`'s settings, set to the document's paper and sides.
pub(super) fn settings(printer: &[u16], (width, height): (f64, f64), sides: Sides) -> anyhow::Result<Devmode> {
    let name = PCWSTR(printer.as_ptr());
    let mut handle = PRINTER_HANDLE::default();
    unsafe { OpenPrinterW(name, &mut handle, None) }
        .with_context(|| format!("the printer {} cannot be opened", String::from_utf16_lossy(&printer[..printer.len() - 1])))?;
    let set = || -> anyhow::Result<Devmode> {
        let bytes = unsafe { DocumentPropertiesW(None, handle, name, None, None, 0) };
        let mut settings = Devmode::zeroed(usize::try_from(bytes).ok().filter(|&b| b >= size_of::<DEVMODEW>()).context("the printer gives no settings")?);
        answered(unsafe { DocumentPropertiesW(None, handle, name, Some(settings.as_mut_ptr()), None, DM_OUT_BUFFER.0) })?;
        // Forms stand upright, in tenths of a millimetre.
        let tenths = |points: f64| (points * 254.0 / 72.0).round();
        let (short, long) = (tenths(width.min(height)), tenths(width.max(height)));
        if long > f64::from(i16::MAX) {
            bail!("the page is larger than any paper");
        }
        let points = |tenths: i32| f64::from(tenths) * 72.0 / 254.0;
        let page = (width.min(height), width.max(height));
        let form = forms(name, &settings)?.into_iter().find(|&(_, across, down)| crate::print::holds((points(across), points(down)), page));
        let fields = settings.fields();
        unsafe {
            let paper = &mut fields.Anonymous1.Anonymous1;
            match form {
                Some((id, ..)) => {
                    paper.dmPaperSize = id as i16;
                    fields.dmFields = (fields.dmFields | DM_PAPERSIZE) & !(DM_PAPERWIDTH | DM_PAPERLENGTH);
                }
                None => {
                    paper.dmPaperSize = DMPAPER_USER as i16;
                    paper.dmPaperWidth = short as i16;
                    paper.dmPaperLength = long as i16;
                    fields.dmFields |= DM_PAPERSIZE | DM_PAPERWIDTH | DM_PAPERLENGTH;
                }
            }
            paper.dmOrientation = if width > height { DMORIENT_LANDSCAPE } else { DMORIENT_PORTRAIT } as i16;
        }
        fields.dmFields |= DM_ORIENTATION;
        if let Some(duplex) = duplex(sides) {
            fields.dmDuplex = duplex;
            fields.dmFields |= DM_DUPLEX;
        }
        // The driver settles what it cannot do.
        let asked = settings.clone();
        answered(unsafe {
            DocumentPropertiesW(None, handle, name, Some(settings.as_mut_ptr()), Some(asked.as_ptr()), DM_IN_BUFFER.0 | DM_OUT_BUFFER.0)
        })?;
        log::info!("print: sides {sides:?}, the driver keeps duplex {}", settings.fields().dmDuplex.0);
        Ok(settings)
    };
    let settings = set();
    unsafe { ClosePrinter(handle) }?;
    settings
}

/// Windows calls long-edge binding vertical.
fn duplex(sides: Sides) -> Option<DEVMODE_DUPLEX> {
    match sides {
        Sides::Printer => None,
        Sides::One => Some(DMDUP_SIMPLEX),
        Sides::Long => Some(DMDUP_VERTICAL),
        Sides::Short => Some(DMDUP_HORIZONTAL),
    }
}

/// The printer's forms: id, then size in tenth millimetres.
fn forms(printer: PCWSTR, settings: &Devmode) -> anyhow::Result<Vec<(u16, i32, i32)>> {
    let count = unsafe { DeviceCapabilitiesW(printer, PCWSTR::null(), DC_PAPERS, None, Some(settings.as_ptr())) };
    let count = usize::try_from(count).context("the printer lists no forms")?;
    let mut ids = vec![0u16; count];
    let mut sizes = vec![POINT::default(); count];
    let listed = [
        unsafe { DeviceCapabilitiesW(printer, PCWSTR::null(), DC_PAPERS, Some(PWSTR(ids.as_mut_ptr())), Some(settings.as_ptr())) },
        unsafe { DeviceCapabilitiesW(printer, PCWSTR::null(), DC_PAPERSIZE, Some(PWSTR(sizes.as_mut_ptr().cast())), Some(settings.as_ptr())) },
    ];
    if listed != [count as i32; 2] {
        bail!("the printer's forms changed while read");
    }
    Ok(ids.into_iter().zip(sizes).map(|(id, size)| (id, size.x, size.y)).collect())
}

/// The default printer's name, ending in a nul.
pub(super) fn default_printer() -> anyhow::Result<Vec<u16>> {
    let mut length = 0;
    // Asked with no buffer, it fails and gives the length.
    let _ = unsafe { GetDefaultPrinterW(None, &mut length) };
    if length == 0 {
        bail!("no default printer is set up");
    }
    let mut name = vec![0u16; length as usize];
    if !unsafe { GetDefaultPrinterW(Some(PWSTR(name.as_mut_ptr())), &mut length) }.as_bool() {
        bail!("the default printer cannot be read: {}", std::io::Error::last_os_error());
    }
    Ok(name)
}

/// Names `printer`, so the dialog keeps our settings.
pub(super) fn names(printer: &[u16]) -> Vec<u8> {
    let driver = wide("winspool");
    let header = (size_of::<DEVNAMES>() / 2) as u16;
    let offsets = [header, header + driver.len() as u16, header + (driver.len() + printer.len()) as u16, 0];
    let words = offsets.into_iter().chain(driver).chain(printer.iter().copied()).chain([0]);
    words.flat_map(u16::to_le_bytes).collect()
}

/// A DEVNAMES block's device name, ending in a nul.
pub(super) fn device(names: HGLOBAL) -> anyhow::Result<Vec<u16>> {
    let base = lock(names)?.cast::<u16>();
    let offset = unsafe { (*base.cast::<DEVNAMES>()).wDeviceOffset } as usize;
    let start = unsafe { base.add(offset) };
    let length = (0..).take_while(|&n| unsafe { *start.add(n) } != 0).count();
    let name = unsafe { std::slice::from_raw_parts(start, length + 1) }.to_vec();
    unlock(names);
    Ok(name)
}

/// A DEVMODE with its driver's tail, kept aligned.
#[derive(Clone)]
pub(super) struct Devmode {
    words: Vec<u64>,
    bytes: usize,
}

impl Devmode {
    fn zeroed(bytes: usize) -> Self {
        Self { words: vec![0; bytes.div_ceil(8)], bytes }
    }

    /// A copy of the DEVMODE in `block`.
    pub(super) fn copied(block: HGLOBAL) -> anyhow::Result<Self> {
        let bytes = unsafe { GlobalSize(block) };
        if bytes < size_of::<DEVMODEW>() {
            bail!("the dialog gave no printer settings");
        }
        let mut settings = Self::zeroed(bytes);
        let source = lock(block)?;
        unsafe { std::ptr::copy_nonoverlapping(source.cast::<u8>(), settings.words.as_mut_ptr().cast(), bytes) };
        unlock(block);
        Ok(settings)
    }

    pub(super) fn as_ptr(&self) -> *const DEVMODEW {
        self.words.as_ptr().cast()
    }

    fn as_mut_ptr(&mut self) -> *mut DEVMODEW {
        self.words.as_mut_ptr().cast()
    }

    fn fields(&mut self) -> &mut DEVMODEW {
        unsafe { &mut *self.as_mut_ptr() }
    }

    pub(super) fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.words.as_ptr().cast(), self.bytes) }
    }
}

/// A movable global block holding `bytes`.
pub(super) fn global(bytes: &[u8]) -> anyhow::Result<HGLOBAL> {
    let block = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) }?;
    let target = lock(block)?;
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), target.cast(), bytes.len()) };
    unlock(block);
    Ok(block)
}

fn lock(block: HGLOBAL) -> anyhow::Result<*mut core::ffi::c_void> {
    let pointer = unsafe { GlobalLock(block) };
    if pointer.is_null() {
        bail!("a printer block cannot be read: {}", std::io::Error::last_os_error());
    }
    Ok(pointer)
}

fn unlock(block: HGLOBAL) {
    // An error here only says the block is now unlocked.
    if let Err(error) = unsafe { GlobalUnlock(block) }
        && error.code().is_err()
    {
        log::warn!("print: a printer block stays locked: {error}");
    }
}

pub(super) fn free(block: HGLOBAL) -> anyhow::Result<()> {
    if block.is_invalid() {
        return Ok(());
    }
    // Freed, it answers null; windows-rs calls that an error.
    match unsafe { GlobalFree(Some(block)) } {
        Err(error) if error.code().is_ok() => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => bail!("a printer block cannot be freed: {}", std::io::Error::last_os_error()),
    }
}

/// DocumentProperties answers IDOK on success.
fn answered(code: i32) -> anyhow::Result<()> {
    if code < 0 {
        bail!("the printer refused its settings");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use windows::{
        Win32::{
            Graphics::{
                Gdi::{DMDUP_HORIZONTAL, DMDUP_SIMPLEX, DMDUP_VERTICAL},
                Printing::{EnumPrintersW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_INFO_4W},
            },
            Storage::Xps::{DC_DUPLEX, DeviceCapabilitiesW},
        },
        core::PCWSTR,
    };

    use super::{duplex, settings};
    use crate::settings::Sides;

    /// An installed printer whose driver prints both sides.
    fn two_sided() -> Option<Vec<u16>> {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut needed, mut count) = (0, 0);
        // Asked with no buffer, it fails and gives the size.
        let _ = unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, None, &mut needed, &mut count) };
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
        let bytes = unsafe { std::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), needed as usize) };
        unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, Some(bytes), &mut needed, &mut count) }.expect("printers listed");
        let infos = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<PRINTER_INFO_4W>(), count as usize) };
        infos.iter().map(|info| unsafe { info.pPrinterName.as_wide() }.iter().copied().chain([0]).collect::<Vec<u16>>()).find(|name| {
            1 == unsafe { DeviceCapabilitiesW(PCWSTR(name.as_ptr()), PCWSTR::null(), DC_DUPLEX, None, None) }
        })
    }

    #[test]
    fn a_two_sided_printer_starts_on_the_sides_chosen() {
        let Some(printer) = two_sided() else {
            return eprintln!("SKIP: no printer here prints both sides");
        };
        let a4 = (595.0, 842.0);
        for (sides, duplex) in [(Sides::Long, DMDUP_VERTICAL), (Sides::Short, DMDUP_HORIZONTAL), (Sides::One, DMDUP_SIMPLEX)] {
            assert_eq!(settings(&printer, a4, sides).expect("settings").fields().dmDuplex, duplex, "{sides:?}");
        }
        assert_eq!(duplex(Sides::Printer), None, "the printer's own stays");
    }
}
