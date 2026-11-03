//! Owner access and protected copies (PDF-11).

use std::{io, path::PathBuf};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    data_display::DescriptionList,
    forms::{Checkbox, Input, TextInput},
    overlays::Dialog as Card,
    primitives::Severity,
};
use ely_office::{Closing as _, Saving as _, Tabs as _, in_window};
use ely_pdf_engine::{Lock, proto::may};
use gpui::{AnyElement, App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Styled as _, TaskExt as _, Window, div};

use crate::redact::beside;
use crate::viewer::Viewer;

/// What a protected copy may allow, with label keys.
const ALLOW: [(u32, &str); 6] = [
    (may::PRINT, "pdf.protect.print"),
    (may::COPY, "pdf.protect.copy"),
    (may::EDIT, "pdf.protect.edit"),
    (may::ANNOTATE, "pdf.protect.annotate"),
    (may::FILL, "pdf.protect.fill"),
    (may::ASSEMBLE, "pdf.protect.assemble"),
];

pub enum Secure {
    Owner { tab: u64, field: Entity<TextInput> },
    Protect { tab: u64, on: bool, user: Entity<TextInput>, owner: Entity<TextInput>, allow: u32 },
}

impl Secure {
    fn tab(&self) -> u64 {
        match self {
            Secure::Owner { tab, .. } | Secure::Protect { tab, .. } => *tab,
        }
    }
}

fn masked(window: &mut Window, cx: &mut Context<Viewer>) -> Entity<TextInput> {
    cx.new(|cx| TextInput::new(window, cx).masked())
}

impl Viewer {
    /// A refusal names what is missing.
    pub(crate) fn failed(&self, error: &io::Error, cx: &mut App) {
        match ely_pdf_engine::refused(error) {
            Some(needed) => self.toast(crate::view::restricted(needed, cx), Severity::Warning, cx),
            None => {
                log::error!("pdf: a step failed ({:?})", error.kind());
                let text = Self::text(cx, "office.error.panel", &[("error", &ely_ui::i18n::error(cx, error))]);
                self.toast_unlogged(text, Severity::Danger, cx);
            }
        }
    }

    fn allowed(&self, cx: &App) -> Option<(u64, u32)> {
        let tab = self.active_tab()?;
        Some((tab.id, tab.view.read(cx).allowed))
    }

    pub fn ask_owner(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((tab, allowed)) = self.allowed(cx) else {
            return self.no_document(cx);
        };
        if allowed & may::OWNER != 0 {
            return self.toast(Self::text(cx, "pdf.owner.already", &[]), Severity::Info, cx);
        }
        self.secure = Some(Secure::Owner { tab, field: masked(window, cx) });
        cx.notify();
    }

    pub fn ask_protect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((tab, allowed)) = self.allowed(cx) else {
            return self.no_document(cx);
        };
        if allowed & may::OWNER == 0 {
            return self.toast(crate::view::restricted("owner", cx), Severity::Warning, cx);
        }
        let all = ALLOW.iter().fold(0, |bits, (bit, _)| bits | bit);
        let (user, owner) = (masked(window, cx), masked(window, cx));
        self.secure = Some(Secure::Protect { tab, on: true, user, owner, allow: all });
        cx.notify();
    }

    pub(crate) fn try_owner(&mut self, cx: &mut Context<Self>) {
        let Some(Secure::Owner { tab, field }) = self.secure.as_ref() else {
            return;
        };
        let (tab, password, asked) = (*tab, field.read(cx).text().to_string(), field.clone());
        let name = self.tab(tab).map(|open| open.name.to_string()).unwrap_or_default();
        self.read_in(
            tab,
            move |worker, doc| {
                let worker = worker.get()?;
                Ok(worker.owner(*doc, &password)?.then_some(worker.permissions(*doc)?))
            },
            move |viewer, done, _, cx| match done {
                Ok(Some(bits)) => {
                    // Only this dialog closes; a newer one stays.
                    if matches!(&viewer.secure, Some(Secure::Owner { field, .. }) if *field == asked) {
                        viewer.secure = None;
                    }
                    if let Some(open) = viewer.tab(tab) {
                        open.view.update(cx, |view, cx| {
                            view.allowed = bits;
                            cx.notify();
                        });
                    }
                    viewer.toast(Self::text(cx, "pdf.owner.done", &[("name", &name)]), Severity::Success, cx);
                }
                Ok(None) => viewer.toast(Self::text(cx, "pdf.owner.wrong", &[]), Severity::Warning, cx),
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// The lock asked for, or None for no protection.
    pub(crate) fn lock(&self, cx: &App) -> Option<Option<Lock>> {
        let Some(Secure::Protect { on, user, owner, allow, .. }) = self.secure.as_ref() else {
            return None;
        };
        let (user, owner) = (user.read(cx).text().to_string(), owner.read(cx).text().to_string());
        match on {
            false => Some(None),
            true if owner.is_empty() || owner == user => None,
            true => Some(Some(Lock { user, owner, allow: *allow })),
        }
    }

    /// Asks where the protected copy goes, then writes it.
    fn choose_protected(&mut self, cx: &mut Context<Self>) {
        let Some(lock) = self.lock(cx) else {
            return;
        };
        let Some(tab) = self.secure.take().map(|secure| secure.tab()) else {
            return;
        };
        cx.notify();
        let Some(open) = self.tab(tab) else {
            return;
        };
        let Some(dir) = ely_office::folder(open) else {
            return self.panel_failed(&ely_ui::i18n::text(cx, "error.no_documents", &[]), cx);
        };
        let tail = Self::text(cx, if lock.is_some() { "pdf.protect.tail" } else { "pdf.protect.bare" }, &[]);
        let chosen = cx.prompt_for_new_path(&dir, Some(&beside(&open.name, &tail)));
        cx.spawn(async move |this, cx| {
            let chosen = match chosen.await {
                Ok(Ok(path)) => Ok(path),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            in_window(&this, cx, |viewer, _, cx| match chosen {
                Ok(Some(target)) => viewer.write_protected(tab, lock, target, cx),
                Ok(None) => log::info!("pdf: the protect panel was cancelled"),
                Err(error) => viewer.panel_failed(&error, cx),
            })
        })
        .detach_and_log_err(cx);
    }

    pub(crate) fn write_protected(&mut self, tab: u64, lock: Option<Lock>, target: PathBuf, cx: &mut Context<Self>) {
        if !self.settled(tab, cx) {
            return self.when_settled(tab, move |viewer, _, cx| viewer.write_protected(tab, lock, target, cx));
        }
        let Some(open) = self.tab(tab) else {
            return;
        };
        let (shown, opened) = (ely_office::files::file_name(&target), target.clone());
        match self.holder(&target) {
            Ok(None) => {}
            Ok(Some(_)) => return self.toast(Self::text(cx, "pdf.extract.taken", &[("name", &shown)]), Severity::Danger, cx),
            Err(error) => return self.failed(&error, cx),
        }
        let (work, path, revision) = (open.work.join("protect.pdf"), open.session.path().map(PathBuf::from), open.session.revision());
        log::info!("pdf: writes a {} copy", if lock.is_some() { "protected" } else { "unprotected" });
        self.read_in(
            tab,
            move |worker, doc| {
                worker.get()?.protect(*doc, &work, lock.as_ref())?;
                let bytes = std::fs::read(&work)?;
                std::fs::remove_file(&work)?;
                ely_files::export(&target, path.as_deref(), &bytes, revision).map(|_| ())
            },
            move |viewer, done, _, cx| match done {
                Ok(()) => viewer.toast_output(Self::text(cx, "pdf.protect.done", &[("name", &shown)]), opened.clone(), cx),
                Err(error) => viewer.failed(&error, cx),
            },
            cx,
        );
    }

    /// The owner or protect dialog, while open.
    pub fn secure_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let secure = self.secure.as_ref()?;
        let this = cx.entity();
        let name = self.tab(secure.tab())?.name.to_string();
        let label = |key: &str| Self::text(cx, key, &[]);
        let closed = this.clone();
        let dismiss = move |_: &mut Window, cx: &mut App| {
            closed.update(cx, |viewer, cx| {
                viewer.secure = None;
                cx.notify();
            })
        };
        let card = match secure {
            Secure::Owner { field, .. } => {
                let go = label("pdf.owner.go");
                let rows = DescriptionList::new().item(label("pdf.owner.field"), div().w_full().child(Input::new(field)));
                Card::new("pdf-owner", Self::text(cx, "pdf.owner.title", &[("name", &name)]), dismiss).child(rows).action(ely_office::cancel(cx)).action(
                    move |_| {
                        Button::new("pdf-owner-go", go)
                            .variant(ButtonVariant::Primary)
                            .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.try_owner(cx)))
                    },
                )
            }
            Secure::Protect { on, user, owner, allow, .. } => {
                let ready = self.lock(cx).is_some();
                let go = label("pdf.removal.go");
                let detail = label(if ready { "pdf.protect.detail" } else { "pdf.protect.needs_owner" });
                let title = Self::text(cx, "pdf.protect.title", &[("name", &name)]);
                let body = self.protect_rows(*on, user, owner, *allow, &this, cx);
                let card = Card::new("pdf-protect", title, dismiss)
                    .detail(detail)
                    .child(body);
                card.action(ely_office::cancel(cx)).action(move |_| {
                    Button::new("pdf-protect-go", go)
                        .variant(ButtonVariant::Primary)
                        .disabled(!ready)
                        .on_click(move |_, _, cx| this.update(cx, |viewer, cx| viewer.choose_protected(cx)))
                })
            }
        };
        Some(card.into_any_element())
    }

    fn protect_rows(&self, on: bool, user: &Entity<TextInput>, owner: &Entity<TextInput>, allow: u32, this: &Entity<Self>, cx: &mut Context<Self>) -> AnyElement {
        type Change = Box<dyn Fn(&mut bool, &mut u32, bool)>;
        let set = |change: Change| {
            let this = this.clone();
            move |value: bool, _: &mut Window, cx: &mut App| {
                this.update(cx, |viewer, cx| {
                    if let Some(Secure::Protect { on, allow, .. }) = viewer.secure.as_mut() {
                        change(on, allow, value);
                    }
                    cx.notify();
                })
            }
        };
        let mut body = div().flex().flex_col().gap_2().child(
            Checkbox::new("pdf-protect-on", on)
                .label(Self::text(cx, "pdf.protect.on", &[]))
                .on_change(set(Box::new(|on, _, value| *on = value))),
        );
        if on {
            let rows = DescriptionList::new()
                .item(Self::text(cx, "pdf.protect.user", &[]), div().w_full().child(Input::new(user)))
                .item(Self::text(cx, "pdf.protect.owner", &[]), div().w_full().child(Input::new(owner)));
            body = body.child(rows);
            for (at, (bit, key)) in ALLOW.iter().enumerate() {
                let bit = *bit;
                body = body.child(
                    Checkbox::new(("pdf-protect-allow", at), allow & bit != 0)
                        .label(Self::text(cx, key, &[]))
                        .on_change(set(Box::new(move |_, allow, value| *allow = if value { *allow | bit } else { *allow & !bit }))),
                );
            }
        }
        body.into_any_element()
    }
}
