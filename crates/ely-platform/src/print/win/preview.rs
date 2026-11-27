//! Windows: the print copy's pages as pictures to preview.

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use windows::{Data::Pdf::PdfPageRenderOptions, Graphics::Imaging::BitmapEncoder};

use super::{open, rendered, winrt};

/// Each picture's width, in pixels asked.
const WIDE: u32 = 480;

/// One page to preview: its picture and size in points.
#[derive(Clone, Debug)]
pub struct Sheet {
    pub picture: PathBuf,
    pub size: (f32, f32),
}

/// Draws each page of `pdf` into `folder` as a PNG.
pub async fn preview_pages(pdf: &Path, folder: &Path) -> anyhow::Result<Vec<Sheet>> {
    let (pdf, folder) = (std::path::absolute(pdf)?, folder.to_path_buf());
    super::super::off(move || winrt(|| pages(&pdf, &folder))).await
}

fn pages(pdf: &Path, folder: &Path) -> anyhow::Result<Vec<Sheet>> {
    std::fs::create_dir_all(folder).with_context(|| format!("{} cannot be made", folder.display()))?;
    let document = open(pdf)?;
    let points = |dips: f32| dips * 72.0 / 96.0;
    let sheets = (0..document.PageCount()?)
        .map(|n| {
            let page = document.GetPage(n)?;
            let size = page.Size()?;
            let options = PdfPageRenderOptions::new()?;
            options.SetDestinationWidth(WIDE)?;
            let png = rendered(&page, &options, BitmapEncoder::PngEncoderId()?)?;
            let picture = folder.join(format!("page-{}.png", n + 1));
            std::fs::write(&picture, png).with_context(|| format!("{} cannot be written", picture.display()))?;
            Ok(Sheet { picture, size: (points(size.Width), points(size.Height)) })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    log::info!("print: {} pages drawn to preview", sheets.len());
    Ok(sheets)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{WIDE, preview_pages};

    #[test]
    fn each_page_becomes_a_picture_at_its_size() {
        let pdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/pdf/sizes.pdf");
        let dir = tempfile::tempdir().expect("a scratch folder");
        let sheets = futures::executor::block_on(preview_pages(&pdf, &dir.path().join("preview"))).expect("drawn");
        assert_eq!(sheets.len(), 2, "two Letter pages");
        for sheet in sheets {
            let (width, height) = sheet.size;
            assert!((width - 612.0).abs() < 1.0 && (height - 792.0).abs() < 1.0, "{:?}", sheet.size);
            let png = std::fs::read(&sheet.picture).expect("the picture");
            assert!(png.starts_with(b"\x89PNG"), "a PNG");
            let across = u32::from_be_bytes(png[16..20].try_into().expect("four bytes"));
            assert!(across.abs_diff(WIDE) <= 4, "{across} pixels across");
        }
    }
}
