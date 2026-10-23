//! macOS: PDFKit's print operation and panel.

use std::{ffi::c_void, path::Path};

use objc2_app_kit::NSPrintInfo;

use crate::settings::Sides;

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn PMSetDuplex(settings: *mut c_void, duplex: u32) -> i32;
    #[cfg(test)]
    fn PMGetDuplex(settings: *mut c_void, duplex: *mut u32) -> i32;
}

/// Shows the print panel for `pdf`; false when cancelled.
pub async fn print_pdf(pdf: &Path, sides: Sides) -> anyhow::Result<bool> {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSPaperOrientation, NSPrintJobSavingURL, NSPrintSaveJob};
    use objc2_foundation::{NSCopying, NSSize, NSString, NSURL};
    use objc2_pdf_kit::{PDFDisplayBox, PDFDocument, PDFPrintScalingMode};

    let mtm =
        MainThreadMarker::new().ok_or_else(|| anyhow::anyhow!("printing needs the main thread"))?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(&pdf.to_string_lossy()));
    let document = unsafe { PDFDocument::initWithURL(PDFDocument::alloc(), &url) }
        .ok_or_else(|| anyhow::anyhow!("{} is no readable PDF", pdf.display()))?;
    let sizes: Vec<(f64, f64)> = (0..unsafe { document.pageCount() })
        .filter_map(|n| unsafe { document.pageAtIndex(n) })
        .map(|page| {
            let size = unsafe { page.boundsForBox(PDFDisplayBox::MediaBox) }.size;
            // A page turned a quarter shows its other side up.
            match unsafe { page.rotation() }.rem_euclid(180) == 90 {
                true => (size.height, size.width),
                false => (size.width, size.height),
            }
        })
        .collect();
    let (width, height) = super::paper_for(&sizes).ok_or_else(|| anyhow::anyhow!("{} has no pages", pdf.display()))?;
    let paper = NSSize::new(width, height);
    if let Some(scale) = crate::env::value("ELY_PRINT_SHARED_SCALE") {
        // Tests start from a shared scale the copy must undo.
        NSPrintInfo::sharedPrintInfo().setScalingFactor(scale.parse()?);
    }
    // The PDF already holds the document's paper, margins and scale.
    let info = NSPrintInfo::sharedPrintInfo().copy();
    info.setScalingFactor(1.0);
    info.setOrientation(match paper.width > paper.height {
        true => NSPaperOrientation::Landscape,
        false => NSPaperOrientation::Portrait,
    });
    info.setPaperSize(paper);
    info.setTopMargin(0.0);
    info.setBottomMargin(0.0);
    info.setLeftMargin(0.0);
    info.setRightMargin(0.0);
    two_sided(&info, sides)?;
    log::info!("print: paper {}x{} points", paper.width, paper.height);
    let operation = unsafe {
        document.printOperationForPrintInfo_scalingMode_autoRotate(
            Some(&info),
            PDFPrintScalingMode::PageScaleNone,
            true,
            mtm,
        )
    }
    .ok_or_else(|| anyhow::anyhow!("no print operation for {}", pdf.display()))?;
    // Tests read the real job: saved to a file.
    let saved = crate::env::value("ELY_PRINT_TO");
    if let Some(to) = &saved {
        let info = operation.printInfo();
        info.setJobDisposition(unsafe { NSPrintSaveJob });
        let to = NSURL::fileURLWithPath(&NSString::from_str(to));
        unsafe { info.dictionary().insert(NSPrintJobSavingURL, &to) };
    }
    operation.setShowsPrintPanel(saved.is_none());
    operation.setShowsProgressPanel(saved.is_none());
    let printed = operation.runOperation();
    log::info!("print: the panel closed, printed {printed}");
    Ok(printed)
}

/// PrintCore calls long-edge binding no tumble.
fn two_sided(info: &NSPrintInfo, sides: Sides) -> anyhow::Result<()> {
    let duplex = match sides {
        Sides::Printer => return Ok(()),
        Sides::One => 1,
        Sides::Long => 2,
        Sides::Short => 3,
    };
    let status = unsafe { PMSetDuplex(info.PMPrintSettings().as_ptr(), duplex) };
    if status != 0 {
        anyhow::bail!("macOS refused the sides {sides:?} ({status})");
    }
    info.updateFromPMPrintSettings();
    log::info!("print: sides {sides:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use objc2_app_kit::NSPrintInfo;
    use objc2_foundation::NSCopying;

    use super::{PMGetDuplex, two_sided};
    use crate::settings::Sides;

    /// PrintCore's answer, then the mode read.
    fn duplex(info: &NSPrintInfo) -> (i32, u32) {
        let mut duplex = 0;
        (unsafe { PMGetDuplex(info.PMPrintSettings().as_ptr(), &mut duplex) }, duplex)
    }

    #[test]
    fn the_panel_starts_on_the_sides_chosen() {
        for (sides, mode) in [(Sides::Long, 2), (Sides::Short, 3), (Sides::One, 1)] {
            let info = NSPrintInfo::sharedPrintInfo().copy();
            two_sided(&info, sides).expect("set");
            // A copy, as the print operation takes one.
            assert_eq!(duplex(&info.copy()), (0, mode), "{sides:?}");
        }
        let info = NSPrintInfo::sharedPrintInfo().copy();
        // Unset reads as kPMKeyNotFound; it must stay unset.
        let own = duplex(&info);
        two_sided(&info, Sides::Printer).expect("left");
        assert_eq!(duplex(&info), own, "the printer's own stays");
    }
}
