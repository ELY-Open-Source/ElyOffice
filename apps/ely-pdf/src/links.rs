//! Links: followed, drawn, retargeted, deleted (PDF-07).

use ely_pdf_engine::{Link, Target};
use gpui::Context;

use crate::view::{Failed, PdfView};

/// A target as typed: a page number or an address.
pub fn parse(typed: &str, pages: usize) -> Option<Target> {
    let typed = typed.trim();
    match typed.parse::<usize>() {
        Ok(n) => (1..=pages).contains(&n).then(|| Target::Page(n - 1, 0.0)),
        Err(_) => ely_ui::outward(typed).ok().map(Target::Web),
    }
}

/// A link's target as a person types it.
pub fn typed(link: &Link) -> String {
    match link.page {
        Some(page) => (page + 1).to_string(),
        None => link.uri.clone(),
    }
}

impl PdfView {
    /// Goes where `link` points; only web and mail leave.
    pub fn follow(&mut self, link: &Link, cx: &mut Context<Self>) {
        match link.page {
            Some(page) => {
                log::info!("pdf: follows a link to page {}", page + 1);
                self.show(page, link.y, cx)
            }
            None => match ely_ui::outward(&link.uri) {
                Ok(address) => {
                    log::info!("pdf: opens a link outside");
                    cx.open_url(&address)
                }
                Err(notice) => {
                    let text = crate::viewer::Viewer::text(cx, notice, &[]);
                    cx.emit(Failed(text.to_string()))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_is_a_page_in_range_or_a_web_or_mail_address() {
        assert_eq!(parse(" 3 ", 5), Some(Target::Page(2, 0.0)));
        assert_eq!(parse("9", 5), None);
        assert_eq!(parse("0", 5), None);
        // Written as the opener gets it.
        assert_eq!(parse("https://a.b", 5), Some(Target::Web("https://a.b/".into())));
        assert_eq!(parse("https://a.b/x y", 5), Some(Target::Web("https://a.b/x%20y".into())));
        assert_eq!(parse("https://a b/", 5), None);
        assert_eq!(parse("javascript:x", 5), None);
        assert_eq!(parse("file:///etc", 5), None);
    }
}
