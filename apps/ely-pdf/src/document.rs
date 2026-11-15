//! The Document menu (PDF-07 to PDF-12).

use gpui::{App, Context, Div, InteractiveElement as _, Menu, MenuItem, actions};

use crate::content::Tool;
use crate::viewer::Viewer;

actions!(
    pdf,
    [
        DocumentProperties,
        RecognizeText,
        AddBookmark,
        RenameBookmark,
        DeleteBookmark,
        AddLink,
        AttachFile,
        SaveAttachment,
        RemoveAttachment,
        RedactText,
        RedactArea,
        ClearRedactions,
        ApplyRedactions,
        CleanDocument,
        OwnerPassword,
        ProtectDocument,
        SignDocument,
        AddVisibleSignature,
        ShowSignatures,
        CloseSignatures,
        ShowRepairReport
    ]
);

pub fn menu(cx: &App) -> Menu {
    let label = |key: &str| Viewer::text(cx, key, &[]);
    Menu::new(label("menu.document")).items([
        MenuItem::action(label("pdf.props.open"), DocumentProperties),
        MenuItem::action(label("pdf.ocr.open"), RecognizeText),
        MenuItem::separator(),
        MenuItem::action(label("pdf.mark.add"), AddBookmark),
        MenuItem::action(label("pdf.tool.link"), AddLink),
        MenuItem::action(label("pdf.file.attach"), AttachFile),
        MenuItem::separator(),
        MenuItem::action(label("pdf.redact.text"), RedactText),
        MenuItem::action(label("pdf.redact.area"), RedactArea),
        MenuItem::action(label("pdf.redact.clear"), ClearRedactions),
        MenuItem::action(label("pdf.redact.apply"), ApplyRedactions),
        MenuItem::action(label("pdf.clean.open"), CleanDocument),
        MenuItem::separator(),
        MenuItem::action(label("pdf.owner.open"), OwnerPassword),
        MenuItem::action(label("pdf.protect.open"), ProtectDocument),
        MenuItem::action(label("pdf.sign.show"), ShowSignatures),
        MenuItem::action(label("pdf.repair.open"), ShowRepairReport),
        MenuItem::action(label("pdf.sign.open"), SignDocument),
        MenuItem::action(label("pdf.visible.open"), AddVisibleSignature),
    ])
}

/// Wires the Document actions onto the root.
pub fn on(root: Div, cx: &mut Context<Viewer>) -> Div {
    root.on_action(cx.listener(|v, _: &DocumentProperties, window, cx| v.ask_properties(window, cx)))
        .on_action(cx.listener(|v, _: &RecognizeText, window, cx| v.ask_ocr(window, cx)))
        .on_action(cx.listener(|v, _: &AddBookmark, window, cx| v.ask_bookmark(true, window, cx)))
        .on_action(cx.listener(|v, _: &RenameBookmark, window, cx| v.ask_bookmark(false, window, cx)))
        .on_action(cx.listener(|v, _: &DeleteBookmark, _, cx| v.delete_bookmark(cx)))
        .on_action(cx.listener(|v, _: &AddLink, _, cx| v.toggle_tool(Tool::Link, cx)))
        .on_action(cx.listener(|v, _: &AttachFile, _, cx| v.attach_file(cx)))
        .on_action(cx.listener(|v, _: &SaveAttachment, _, cx| v.save_attachment(cx)))
        .on_action(cx.listener(|v, _: &RemoveAttachment, _, cx| v.remove_attachment(cx)))
        .on_action(cx.listener(|v, _: &RedactText, _, cx| v.toggle_tool(Tool::RedactText, cx)))
        .on_action(cx.listener(|v, _: &RedactArea, _, cx| v.toggle_tool(Tool::RedactArea, cx)))
        .on_action(cx.listener(|v, _: &ClearRedactions, _, cx| v.clear_redactions(cx)))
        .on_action(cx.listener(|v, _: &ApplyRedactions, _, cx| v.ask_redact(cx)))
        .on_action(cx.listener(|v, _: &CleanDocument, _, cx| v.ask_clean(cx)))
        .on_action(cx.listener(|v, _: &OwnerPassword, window, cx| v.ask_owner(window, cx)))
        .on_action(cx.listener(|v, _: &ProtectDocument, window, cx| v.ask_protect(window, cx)))
        .on_action(cx.listener(|v, _: &SignDocument, window, cx| v.ask_sign(window, cx)))
        .on_action(cx.listener(|v, _: &AddVisibleSignature, window, cx| v.ask_visible(window, cx)))
        .on_action(cx.listener(|v, _: &ShowSignatures, _, cx| v.show_signatures(true, cx)))
        .on_action(cx.listener(|v, _: &CloseSignatures, _, cx| v.show_signatures(false, cx)))
        .on_action(cx.listener(|v, _: &ShowRepairReport, _, cx| v.show_repair(true, cx)))
}
