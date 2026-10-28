//! The add-account dialog (MAIL-01, 02).

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    data_display::DescriptionList,
    feedback::{Banner, InlineMessage},
    forms::{Input, PasswordInput, Switch},
    overlays::Dialog,
    primitives::Severity,
};
use ely_mail_engine::{Auth, Error, Security};
use gpui::{
    AnyElement, App, Context, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, WeakEntity, Window, div,
};

use crate::{
    adding::{Adding, Step},
    kinds::{APPLE_ACCOUNT, GOOGLE_APP_PASSWORDS, KINDS, Kind},
    mail::{Mail, text},
    view::auth,
};

type Edit = fn(&mut Adding, &str, &mut App);

/// A segmented choice whose change edits the dialog.
fn choice(
    this: &WeakEntity<Mail>,
    id: &'static str,
    picked: &str,
    options: impl IntoIterator<Item = (&'static str, SharedString)>,
    edit: Edit,
) -> SegmentedControl {
    let this = this.clone();
    options
        .into_iter()
        .fold(
            SegmentedControl::new(id, picked.to_owned()),
            |control, (key, label)| control.segment(key, label, None),
        )
        .on_change(move |key, _, cx| {
            this.update(cx, |mail, cx| {
                if let Some(adding) = &mut mail.adding {
                    edit(adding, key, cx);
                    cx.notify();
                }
            })
            .ok();
        })
}

fn security_key(security: Security) -> &'static str {
    match security {
        Security::Tls => "tls",
        Security::StartTls => "starttls",
    }
}

fn security_of(key: &str) -> Security {
    match key {
        "tls" => Security::Tls,
        _ => Security::StartTls,
    }
}

fn auth_key(auth: Auth) -> &'static str {
    match auth {
        Auth::Password => "password",
        Auth::Google => "google",
        Auth::Microsoft => "microsoft",
    }
}

impl Mail {
    pub fn add_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let adding = self.adding.as_ref()?;
        let this = cx.weak_entity();
        let dismiss = {
            let this = this.clone();
            move |_: &mut Window, cx: &mut App| {
                this.update(cx, |mail, cx| mail.close_adding(cx)).ok();
            }
        };
        let kinds = KINDS.map(|kind| {
            (
                kind.key(),
                text(cx, &format!("mail.kind.{}", kind.key()), &[]),
            )
        });
        let kinds = choice(
            &this,
            "mail-add-kind",
            adding.kind.key(),
            kinds,
            |adding, key, cx| {
                if let Some(kind) = Kind::from_key(key) {
                    adding.set_kind(kind, cx);
                }
            },
        );
        let oauth = adding.auth() != Auth::Password;
        let unavailable = match adding.auth().provider() {
            Some(Err(Error::Unavailable(name))) => {
                Some(text(cx, &format!("mail.add.unavailable.{name}"), &[]))
            }
            _ => None,
        };
        let cancel = text(cx, "mail.cancel", &[]);
        let (go_key, go): (&str, fn(&mut Mail, &mut Context<Mail>)) = match &adding.step {
            Step::Checked { checked, .. } if checked.passed() => ("mail.add.save", Mail::save),
            _ if oauth => ("mail.add.sign_in", Mail::sign_in),
            _ => ("mail.add.check", Mail::check),
        };
        let go_label = text(cx, go_key, &[("method", &auth(adding.auth(), cx))]);
        let busy = adding.busy();
        let blocked = busy || (unavailable.is_some() && oauth);
        let dialog = Dialog::new("mail-add", text(cx, "mail.add.title", &[]), dismiss)
            .child(kinds)
            .children(
                unavailable
                    .map(|words| Banner::new("mail-add-unavailable", Severity::Danger, words)),
            )
            .child(self.add_fields(adding, &this, cx))
            .children(status(adding, cx))
            .action(move |close| {
                Button::new("mail-add-cancel", cancel)
                    .on_click(move |_, window, cx| close(window, cx))
            })
            .action(move |_| {
                Button::new("mail-add-go", go_label)
                    .variant(ButtonVariant::Primary)
                    .loading(busy)
                    .disabled(blocked)
                    .on_click(move |_, _, cx| {
                        this.update(cx, go).ok();
                    })
            });
        Some(dialog.into_any_element())
    }

    fn add_fields(&self, adding: &Adding, this: &WeakEntity<Mail>, cx: &App) -> AnyElement {
        let label = |key: &str| text(cx, key, &[]);
        let field = |input| div().w_full().child(Input::new(input));
        let fields = &adding.fields;
        let password = div().w_full().child(PasswordInput::new(&fields.password));
        match adding.kind {
            Kind::Microsoft => DescriptionList::new()
                .item(label("mail.add.address_hint"), field(&fields.address))
                .into_any_element(),
            Kind::Gmail | Kind::ICloud => {
                let (words, page) = match adding.kind {
                    Kind::Gmail => ("mail.gmail", GOOGLE_APP_PASSWORDS),
                    _ => ("mail.icloud", APPLE_ACCOUNT),
                };
                let open = Button::new("mail-add-page", label(&format!("{words}.open")))
                    .on_click(move |_, _, cx| cx.open_url(page));
                let rows = DescriptionList::new()
                    .item(label("mail.add.address"), field(&fields.address))
                    .item(label(&format!("{words}.password")), password);
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(label(&format!("{words}.steps")))
                    .child(div().debug_selector(|| "mail-add-page".into()).child(open))
                    .child(rows)
                    .into_any_element()
            }
            Kind::Imap | Kind::Pop => {
                let auths = [Auth::Password, Auth::Microsoft]
                    .map(|each| (auth_key(each), auth(each, cx)));
                let methods = choice(
                    this,
                    "mail-add-auth",
                    auth_key(adding.auth),
                    auths,
                    |adding, key, _| {
                        adding.stop();
                        adding.step = Step::Editing;
                        adding.auth = match key {
                            "microsoft" => Auth::Microsoft,
                            _ => Auth::Password,
                        };
                    },
                );
                let securities = |outgoing: bool, picked: Security| {
                    let options = [Security::Tls, Security::StartTls].map(|each| {
                        (
                            security_key(each),
                            label(&format!("mail.security.{}", security_key(each))),
                        )
                    });
                    let id = if outgoing {
                        "mail-add-out-security"
                    } else {
                        "mail-add-in-security"
                    };
                    let edit: Edit = if outgoing {
                        |adding, key, cx| adding.set_security(true, security_of(key), cx)
                    } else {
                        |adding, key, cx| adding.set_security(false, security_of(key), cx)
                    };
                    choice(this, id, security_key(picked), options, edit)
                };
                let mut rows = DescriptionList::new()
                    .item(label("mail.add.address"), field(&fields.address))
                    .item(label("mail.auth.title"), methods);
                if adding.auth == Auth::Password {
                    rows = rows
                        .item(label("mail.add.user"), field(&fields.user))
                        .item(label("mail.add.password"), password);
                }
                if adding.kind == Kind::Pop {
                    let this = this.clone();
                    let keep = Switch::new("mail-add-keep", adding.keep)
                        .name(label("mail.add.keep"))
                        .on_change(move |on, _, cx| {
                            this.update(cx, |mail, cx| {
                                if let Some(adding) = &mut mail.adding {
                                    adding.set_keep(on);
                                    cx.notify();
                                }
                            })
                            .ok();
                        });
                    rows = rows.item(label("mail.add.keep"), keep);
                }
                rows.item(label("mail.add.incoming"), field(&fields.incoming))
                    .item(label("mail.add.port"), field(&fields.incoming_port))
                    .item(
                        label("mail.add.security"),
                        securities(false, adding.incoming_security),
                    )
                    .item(label("mail.add.outgoing"), field(&fields.outgoing))
                    .item(label("mail.add.port"), field(&fields.outgoing_port))
                    .item(
                        label("mail.add.security"),
                        securities(true, adding.outgoing_security),
                    )
                    .into_any_element()
            }
        }
    }
}

/// Each server's answer, or what is under way.
fn status(adding: &Adding, cx: &App) -> Option<AnyElement> {
    let line =
        |severity, words: SharedString| InlineMessage::new(severity, words).into_any_element();
    let said = |key: &str, result: &ely_mail_engine::Result<()>| match result {
        Ok(()) => line(
            Severity::Success,
            text(cx, key, &[("result", &text(cx, "mail.check.passed", &[]))]),
        ),
        Err(error) => line(
            Severity::Danger,
            text(cx, key, &[("result", &failure(error, cx))]),
        ),
    };
    Some(match &adding.step {
        Step::Editing => return None,
        Step::Signing => line(Severity::Info, text(cx, "mail.add.signing", &[])),
        Step::Checking => line(Severity::Info, text(cx, "mail.add.checking", &[])),
        Step::Saving => line(Severity::Info, text(cx, "mail.add.saving", &[])),
        Step::Failed(words) => line(Severity::Danger, words.clone()),
        Step::Checked { checked, .. } => div()
            .flex()
            .flex_col()
            .gap_1()
            .child(said("mail.check.incoming", &checked.incoming))
            .child(said("mail.check.outgoing", &checked.outgoing))
            .children(advice(adding.kind, checked).map(|key| line(Severity::Info, text(cx, key, &[]))))
            .into_any_element(),
    })
}

/// What to do when a provider refuses the password.
fn advice(kind: Kind, checked: &ely_mail_engine::Checked) -> Option<&'static str> {
    let refused = [&checked.incoming, &checked.outgoing]
        .into_iter()
        .any(|result| matches!(result, Err(Error::Rejected(_))));
    (refused && kind == Kind::Gmail).then_some("mail.gmail.refused")
}

/// Outlook.com's words for an account it holds.
const OUTLOOK_HELD: &str = "User is authenticated but not connected";

/// A server's refusal in the reader's language, its words kept.
pub fn failure(error: &Error, cx: &App) -> SharedString {
    let (key, detail) = match error {
        Error::Connect(err) => ("mail.error.connect", err.to_string()),
        Error::Tls(detail) => ("mail.error.tls_failed", detail.clone()),
        Error::Rejected(detail) if detail.contains(OUTLOOK_HELD) => {
            ("mail.error.outlook_held", detail.clone())
        }
        Error::Rejected(detail) => ("mail.error.rejected", detail.clone()),
        Error::Server(detail) => ("mail.error.server", detail.clone()),
        Error::Protocol(detail) => ("mail.error.protocol", detail.clone()),
        Error::Timeout(step) => ("mail.error.timeout", (*step).to_owned()),
        other => ("mail.error.other", other.to_string()),
    };
    text(cx, key, &[("detail", &detail)])
}
