//! Signatures: checked beside the pages, and signed copies (PDF-12, QA-15).

use std::{io, path::PathBuf};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    feedback::EmptyState,
    forms::{Input, TextInput},
    overlays::Dialog as Card,
    primitives::{IconName, Severity},
    theme::ActiveTheme as _,
};
use ely_office::{Closing as _, Saving as _, Tabs as _, in_window};
use ely_pdf_engine::{Digest, Signature, Signed, Signing};
use ely_platform::trust::{self, Trust};
use gpui::{AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, StatefulInteractiveElement as _, Styled as _, TaskExt as _, Window, div};

use crate::redact::beside;
use crate::view::{PdfView, Unread};
use crate::viewer::Viewer;

/// A signature field and the system's word on its certificate.
pub struct Checked {
    pub field: Signature,
    pub trust: Option<Result<Trust, String>>,
}

/// Where a new signature goes, set by the sign tool.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignAt {
    pub page: usize,
    /// An empty field's object, or a new field.
    pub object: Option<u32>,
    pub rect: [f32; 4],
    pub version: u64,
}

/// The certificate and words for a new signature.
pub struct Signer {
    pub pfx: Option<PathBuf>,
    pub password: Entity<TextInput>,
    pub reason: Entity<TextInput>,
    pub location: Entity<TextInput>,
}

impl PdfView {
    /// Reads and checks the signatures of the file at `path`.
    pub fn ask_signatures(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let failed = self.unread.contains(&Unread::Signatures(path.clone()));
        if self.signatures.as_ref().is_some_and(|(read, _)| *read == path) || self.reading_signatures || failed {
            return;
        }
        self.reading_signatures = true;
        // A save in between makes this read stale: dropped.
        let (doc, asked, saves) = (self.doc, path.clone(), self.saves);
        self.off(
            move |worker| {
                let fields = worker.signatures(doc, &path)?;
                Ok(Iterator::collect::<Vec<_>>(fields.into_iter().map(|field| {
                    let chain = field.signed.as_ref().map(|signed| signed.certs.clone()).filter(|certs| !certs.is_empty());
                    let trust = chain.map(|chain| trust::check(&chain).map_err(|error| error.to_string()));
                    Checked { field, trust }
                })))
            },
            move |view, read: io::Result<Vec<Checked>>, cx| {
                view.reading_signatures = false;
                if view.saves != saves {
                    log::info!("pdf: a signature read from before a save is dropped");
                    return cx.notify();
                }
                match read {
                    Ok(read) => view.signatures = Some((asked, read)),
                    Err(error) => {
                        view.unread.insert(Unread::Signatures(asked));
                        view.fail("pdf.doing.signatures", &error, cx)
                    }
                }
                cx.notify();
            },
            cx,
        );
    }
}

fn integrity(signed: &Signed, cx: &App) -> gpui::SharedString {
    let key = match (signed.digest, signed.later, signed.broken) {
        (Digest::Unreadable, ..) => return Viewer::text(cx, "pdf.sign.unreadable", &[("why", signed.error.as_deref().unwrap_or(""))]),
        (Digest::Changed, ..) => "pdf.sign.changed",
        (Digest::Intact, false, _) => "pdf.sign.intact",
        (Digest::Intact, true, false) => "pdf.sign.later",
        (Digest::Intact, true, true) => "pdf.sign.broken",
    };
    Viewer::text(cx, key, &[])
}

fn trusted(trust: &Option<Result<Trust, String>>, cx: &App) -> gpui::SharedString {
    match trust {
        Some(Ok(Trust::Trusted)) => Viewer::text(cx, "pdf.sign.trusted", &[]),
        Some(Ok(Trust::Untrusted(why))) => Viewer::text(cx, "pdf.sign.untrusted", &[("why", why)]),
        Some(Err(why)) => Viewer::text(cx, "pdf.sign.unchecked", &[("why", why)]),
        None => Viewer::text(cx, "pdf.sign.no_certificate", &[]),
    }
}

/// One signature's facts, each on its own row.
fn facts(checked: &Checked, at: usize, view: &Entity<PdfView>, cx: &App) -> AnyElement {
    let (field, label) = (&checked.field, |key: &str| Viewer::text(cx, key, &[]));
    let mut rows = DescriptionList::new();
    let who = match &field.signed {
        None => {
            rows = rows.item(label("pdf.sign.state"), label("pdf.sign.empty"));
            field.name.clone()
        }
        Some(signed) => {
            rows = rows
                .item(label("pdf.sign.integrity"), integrity(signed, cx))
                .item(label("pdf.sign.trust"), trusted(&checked.trust, cx))
                .item(label("pdf.sign.revoked"), label("pdf.sign.revocation"))
                .item(label("pdf.sign.when"), crate::props::date(&signed.date));
            for (key, value) in [("pdf.sign.reason", &signed.reason), ("pdf.sign.location", &signed.location)] {
                if !value.is_empty() {
                    rows = rows.item(label(key), value.clone());
                }
            }
            if signed.common.is_empty() { field.name.clone() } else { signed.common.clone() }
        }
    };
    let go = field.page.map(|page| {
        let shown = view.clone();
        Button::new(("pdf-sign-page", at), Viewer::text(cx, "pdf.page.label", &[("n", &(page + 1).to_string())]))
            .variant(ButtonVariant::Ghost)
            .on_click(move |_, window, cx| {
                shown.update(cx, |view, cx| view.show(page, 0.0, cx));
                window.dispatch_action(Box::new(crate::document::CloseSignatures), cx);
            })
    });
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().flex().items_center().justify_between().child(div().font_weight(gpui::FontWeight::SEMIBOLD).text_color(cx.theme().colors.fg).child(who)).children(go))
        .child(rows)
        .into_any_element()
}

/// The signatures of `view`: integrity, trust, revocation, apart.
pub fn list(view: &Entity<PdfView>, path: Option<PathBuf>, cx: &mut App) -> AnyElement {
    let Some(path) = path else {
        return EmptyState::new("pdf-signs-none", IconName::File, Viewer::text(cx, "pdf.sign.none", &[])).into_any_element();
    };
    view.update(cx, |view, cx| view.ask_signatures(path.clone(), cx));
    let read = view.read(cx);
    let Some(fields) = read.signatures.as_ref().filter(|(read, _)| *read == path).map(|(_, fields)| fields) else {
        return div().into_any_element();
    };
    if fields.is_empty() {
        return EmptyState::new("pdf-signs-none", IconName::File, Viewer::text(cx, "pdf.sign.none", &[])).into_any_element();
    }
    let blocks: Vec<AnyElement> = fields.iter().enumerate().map(|(at, checked)| facts(checked, at, view, cx)).collect();
    div().id("pdf-signs").flex().flex_col().gap_4().overflow_y_scroll().children(blocks).into_any_element()
}

impl Viewer {
    /// The signatures dialog, while open.
    pub fn signs_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (view, tab) = (self.active_view().filter(|_| self.signs_open)?, self.active_tab()?);
        let title = Self::text(cx, "pdf.sign.list", &[("name", &tab.name)]);
        let this = cx.entity();
        let dismiss = move |_: &mut Window, cx: &mut App| this.update(cx, |viewer, cx| viewer.show_signatures(false, cx));
        let path = tab.session.path().map(PathBuf::from);
        let body = div().max_h(gpui::px(480.0)).overflow_hidden().child(list(&view, path, cx));
        let done = Self::text(cx, "pdf.sign.close", &[]);
        let close = move |close: ely_gpui_component::overlays::Close| {
            Button::new("pdf-signs-close", done).variant(ButtonVariant::Primary).on_click(move |_, window, cx| close(window, cx))
        };
        Some(Card::new("pdf-signs", title, dismiss).child(body).action(close).into_any_element())
    }

    pub fn show_signatures(&mut self, open: bool, cx: &mut Context<Self>) {
        let Some(view) = self.active_view().filter(|_| open) else {
            self.signs_open = false;
            return if open { self.no_document(cx) } else { cx.notify() };
        };
        // Each opening checks the file as it is now.
        view.update(cx, |view, _| view.signatures = None);
        self.signs_open = open;
        cx.notify();
    }

    /// Turns the sign tool on with a fresh certificate form.
    pub fn ask_sign(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = |masked: bool, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| if masked { TextInput::new(window, cx).masked() } else { TextInput::new(window, cx) })
        };
        let (password, reason, location) = (input(true, window, cx), input(false, window, cx), input(false, window, cx));
        self.signer = Some(Signer { pfx: None, password, reason, location });
        self.toggle_tool(crate::content::Tool::Sign, cx);
    }

    fn choose_certificate(&mut self, cx: &mut Context<Self>) {
        self.pick(false, cx, |viewer, path, cx| {
            if let Some(signer) = viewer.signer.as_mut() {
                signer.pfx = Some(path);
            }
            cx.notify();
        });
    }

    fn cancel_sign(&mut self, cx: &mut Context<Self>) {
        if let Some(view) = self.active_view() {
            view.update(cx, |view, cx| {
                view.sign_at = None;
                cx.notify();
            });
        }
    }

    /// Asks where the signed copy goes, then writes it.
    fn choose_signed(&mut self, cx: &mut Context<Self>) {
        let (Some(open), Some(view)) = (self.active_tab(), self.active_view()) else {
            return;
        };
        let Some(at) = view.read(cx).sign_at else {
            return;
        };
        let Some(signer) = self.signer.as_ref() else {
            return;
        };
        let Some(pfx) = signer.pfx.as_ref().and_then(|path| path.to_str()).map(str::to_string) else {
            return;
        };
        let text = |field: &Entity<TextInput>| field.read(cx).text().to_string();
        let how = Signing {
            page: at.page,
            object: at.object,
            rect: at.rect,
            name: String::new(),
            pfx,
            pfx_password: text(&signer.password),
            reason: text(&signer.reason),
            location: text(&signer.location),
        };
        let Some(dir) = ely_office::folder(open) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let tab = open.id;
        let chosen = cx.prompt_for_new_path(&dir, Some(&beside(&open.name, &Self::text(cx, "pdf.sign.tail", &[]))));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |viewer, _, cx| match chosen {
                Ok(Some(target)) => viewer.write_signed(tab, at, how, target, cx),
                Ok(None) => log::info!("pdf: the sign panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    /// Signs into `target` if nothing changed since the drag.
    pub(crate) fn write_signed(&mut self, tab: u64, at: SignAt, mut how: Signing, target: PathBuf, cx: &mut Context<Self>) {
        let (Some(open), Some(view)) = (self.tab(tab), self.tab(tab).map(|tab| tab.view.clone())) else {
            return;
        };
        let read = view.read(cx);
        if read.version != at.version || !read.settled() || read.sign_at != Some(at) {
            return self.toast(Self::text(cx, "pdf.object.changed", &[]), Severity::Warning, cx);
        }
        let (shown, opened) = (ely_office::files::file_name(&target), target.clone());
        match self.holder(&target) {
            Ok(None) => {}
            Ok(Some(_)) => return self.toast(Self::text(cx, "pdf.extract.taken", &[("name", &shown)]), Severity::Danger, cx),
            Err(error) => return self.failed(&error, cx),
        }
        let (work, path, revision) = (open.work.join("signed.pdf"), open.session.path().map(PathBuf::from), open.session.revision());
        log::info!("pdf: writes a signed copy");
        let placed = view.clone();
        self.read_in(
            tab,
            move |worker, doc| {
                let worker = worker.get()?;
                if how.object.is_none() {
                    let names = worker.field_names(*doc)?;
                    how.name = (1..).map(|n| format!("Signature{n}")).find(|name| !names.contains(name)).expect("a free name");
                }
                worker.sign(*doc, &work, &how)?;
                let bytes = std::fs::read(&work)?;
                std::fs::remove_file(&work)?;
                ely_files::export(&target, path.as_deref(), &bytes, revision).map(|_| ())
            },
            move |viewer, done, _, cx| match done {
                Ok(()) => {
                    placed.update(cx, |view, cx| {
                        if view.sign_at == Some(at) {
                            view.sign_at = None;
                            cx.notify();
                        }
                    });
                    viewer.toast_output(Self::text(cx, "pdf.sign.done", &[("name", &shown)]), opened.clone(), cx);
                }
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// The signing form, once a place is chosen.
    pub fn sign_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let signer = self.signer.as_ref()?;
        self.active_view()?.read(cx).sign_at?;
        let this = cx.entity();
        let label = |key: &str| Self::text(cx, key, &[]);
        let chosen = signer.pfx.as_ref().map_or_else(|| label("pdf.sign.no_file"), |path| ely_office::files::file_name(path));
        let pick = this.clone();
        let rows = DescriptionList::new()
            .item(
                label("pdf.sign.certificate"),
                div().flex().gap_2().items_center().child(div().flex_1().child(chosen)).child(
                    Button::new("pdf-sign-pick", label("pdf.sign.choose"))
                        .on_click(move |_, _, cx| pick.update(cx, |viewer, cx| viewer.choose_certificate(cx))),
                ),
            )
            .item(label("pdf.sign.password"), div().w_full().child(Input::new(&signer.password)))
            .item(label("pdf.sign.reason"), div().w_full().child(Input::new(&signer.reason)))
            .item(label("pdf.sign.location"), div().w_full().child(Input::new(&signer.location)));
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| closed.update(cx, |viewer, cx| viewer.cancel_sign(cx));
        let (ready, go) = (signer.pfx.is_some(), label("pdf.removal.go"));
        let card = Card::new("pdf-sign", label("pdf.sign.title"), dismiss)
            .detail(label("pdf.sign.detail"))
            .child(rows)
            .action(ely_office::cancel(cx))
            .action(move |_| {
                Button::new("pdf-sign-go", go)
                    .variant(ButtonVariant::Primary)
                    .disabled(!ready)
                    .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.choose_signed(cx)))
            });
        Some(card.into_any_element())
    }
}
