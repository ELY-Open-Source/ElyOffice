//! Why a step failed, worded: no paths, no system text.

use std::io;

use ely_ipc::{Then, Worded};
use gpui::{App, SharedString};

/// A worded error by its key; others by kind (R20.70).
pub(crate) fn reason(cx: &App, error: &io::Error) -> SharedString {
    if let Some(then) = error.get_ref().and_then(|inner| inner.downcast_ref::<Then>()) {
        let (first, left) = (reason(cx, &then.error), reason(cx, &then.left));
        return ely_ui::i18n::text(cx, "error.then", &[("error", &first), ("left", &left)]);
    }
    if error.get_ref().is_some_and(|inner| inner.is::<Worded>()) {
        return ely_ui::i18n::error(cx, error);
    }
    use io::ErrorKind::{FileTooLarge, NotFound, Other, PermissionDenied, QuotaExceeded, ReadOnlyFilesystem, StorageFull};
    let key = match error.kind() {
        PermissionDenied | ReadOnlyFilesystem => "pdf.error.denied",
        StorageFull | QuotaExceeded | FileTooLarge => "pdf.error.full",
        NotFound => "pdf.error.missing",
        // MuPDF's text may quote a path or the document.
        Other => "pdf.error.engine",
        _ => "pdf.error.other",
    };
    ely_ui::i18n::text(cx, key, &[])
}
