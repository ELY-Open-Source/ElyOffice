//! Prints a PDF through the system print dialog.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

#[cfg(target_os = "linux")]
pub use linux::print_pdf;
#[cfg(target_os = "macos")]
pub use mac::print_pdf;
#[cfg(windows)]
pub use win::{Sheet, preview_pages, print_pdf};

/// Paper sizes round to whole points at worst.
#[cfg(not(target_os = "macos"))]
const ROUNDING: f64 = 0.5;

/// Whether a named paper, short side first, holds a page.
#[cfg(not(target_os = "macos"))]
fn holds((across, down): (f64, f64), (short, long): (f64, f64)) -> bool {
    // A named paper may run 2 mm over, never under.
    let suits = |paper: f64, page: f64| paper >= page - ROUNDING && paper - page <= 2.0 * 72.0 / 25.4;
    suits(across, short) && suits(down, long)
}

/// One paper holding every page, turned as needed.
fn paper_for(sizes: &[(f64, f64)]) -> Option<(f64, f64)> {
    let long = sizes.iter().map(|(w, h)| w.max(*h)).fold(None, |most: Option<f64>, side| Some(most.map_or(side, |m| m.max(side))))?;
    let short = sizes.iter().map(|(w, h)| w.min(*h)).fold(0.0, f64::max);
    // The largest page sets the paper's way up.
    let (w, h) = sizes.iter().copied().fold((0.0, 0.0), |big, size| if size.0 * size.1 > big.0 * big.1 { size } else { big });
    // A square page leaves it to the others.
    let wide = match w == h {
        true => sizes.iter().any(|(w, h)| w > h),
        false => w > h,
    };
    Some(if wide { (long, short) } else { (short, long) })
}

/// Runs `work` on its own thread, off the app's.
#[cfg(not(target_os = "macos"))]
async fn off<T: Send + 'static>(work: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> anyhow::Result<T> {
    let (sender, answer) = futures::channel::oneshot::channel();
    std::thread::Builder::new().name("ely-print".into()).spawn(move || {
        if sender.send(work()).is_err() {
            log::warn!("print: the job ended after its caller left");
        }
    })?;
    answer.await.map_err(|_| anyhow::anyhow!("the print thread ended without an answer"))?
}

#[cfg(test)]
mod tests {
    use super::paper_for;

    #[test]
    fn the_paper_holds_every_page() {
        assert_eq!(paper_for(&[(595.0, 420.0)]), Some((595.0, 420.0)));
        assert_eq!(paper_for(&[(420.0, 595.0), (842.0, 1191.0)]), Some((842.0, 1191.0)));
        // Wide and square pages: neither side may be cut.
        assert_eq!(paper_for(&[(1000.0, 200.0), (600.0, 600.0)]), Some((1000.0, 600.0)));
        assert_eq!(paper_for(&[]), None);
    }
}
