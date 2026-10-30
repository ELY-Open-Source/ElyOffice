//! Contacts: kept, imported, exported, and offered as addresses (C12.19).

use std::{path::PathBuf, rc::Rc};

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    feedback::{Alert, EmptyState},
    forms::{Input, TextInput},
    primitives::{IconName, Severity},
    theme::{ActiveTheme as _, ControlSize},
};
use ely_mail_engine::{Contact, ContactId, Error, vcard};
use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, PathPromptOptions,
    Styled as _, Window, WindowHandle, div, uniform_list,
};

use crate::{
    compose::Compose,
    mail::{Mail, text},
};

/// The contacts as last read.
pub struct People {
    /// Numbers the read; the add fields start over with it.
    pub read: u64,
    pub list: Option<Result<Vec<Contact>, String>>,
}

impl Mail {
    pub fn read_contacts(&mut self, cx: &mut Context<Self>) {
        self.asked += 1;
        let read = self.asked;
        self.people = Some(People { read, list: None });
        self.with_store(cx, |store| store.contacts(), move |mail, list, cx| {
            if let Some(people) = mail.people.as_mut().filter(|each| each.read == read) {
                people.list = Some(list);
            }
            mail.notify(cx);
        });
    }

    /// The contacts a new message offers as addresses.
    pub fn offer_contacts(&self, window: WindowHandle<Compose>, cx: &mut Context<Self>) {
        self.with_store(cx, |store| store.contacts(), move |mail, list, cx| match list {
            Ok(list) => {
                window.update(cx, |compose, _, cx| compose.offer(list, cx)).ok();
            }
            Err(error) => mail.failed("mail.contacts.unread", &error, cx),
        });
    }

    fn add_contact(&mut self, name: String, address: String, cx: &mut Context<Self>) {
        self.with_store(cx, move |store| store.add_contact(&name, &address), |mail, added, cx| match added {
            Ok(_) => mail.read_contacts(cx),
            Err(error) => mail.failed("mail.contacts.unsaved", &error, cx),
        });
    }

    fn remove_contact(&mut self, id: ContactId, cx: &mut Context<Self>) {
        self.with_store(cx, move |store| store.remove_contact(id), |mail, removed, cx| match removed {
            Ok(()) => mail.read_contacts(cx),
            Err(error) => mail.failed("mail.contacts.unsaved", &error, cx),
        });
    }

    fn import_contacts(&mut self, cx: &mut Context<Self>) {
        let options = PathPromptOptions { files: true, directories: false, multiple: false, prompt: None };
        let picked = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let path = match picked.await {
                Ok(Ok(Some(mut paths))) => paths.remove(0),
                Ok(Ok(None)) => return log::info!("mail: importing contacts was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the import panel failed: {error}"),
                Err(error) => return log::error!("mail: the import panel went away: {error}"),
            };
            this.update(cx, |mail, cx| mail.import_from(path, cx)).ok();
        })
        .detach();
    }

    fn import_from(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let work = move |store: &mut ely_mail_engine::Store| {
            let bytes = std::fs::read(&path).map_err(|err| Error::Store(err.to_string()))?;
            let text = String::from_utf8(bytes).map_err(|_| Error::Invalid("the file is not UTF-8".into()))?;
            let cards = vcard::read(text.strip_prefix('\u{feff}').unwrap_or(&text))?;
            store.import_contacts(&cards)
        };
        self.with_store(cx, work, |mail, imported, cx| match imported {
            Ok(done) => {
                let counts = [
                    ("added", done.added.to_string()),
                    ("renamed", done.renamed.to_string()),
                    ("unchanged", done.unchanged.to_string()),
                    ("skipped", done.without_address.to_string()),
                ];
                let counts: Vec<(&str, &str)> = counts.iter().map(|(key, value)| (*key, value.as_str())).collect();
                mail.toast(text(cx, "mail.contacts.imported", &counts), Severity::Success, cx);
                mail.read_contacts(cx);
            }
            Err(error) => mail.failed("mail.contacts.unimported", &error, cx),
        });
    }

    fn export_contacts(&mut self, cx: &mut Context<Self>) {
        let Some(dirs) = directories::UserDirs::new() else {
            return self.failed("mail.contacts.unexported", "no home folder", cx);
        };
        let dir = dirs.document_dir().unwrap_or(dirs.home_dir()).to_path_buf();
        let chosen = cx.prompt_for_new_path(&dir, Some("contacts.vcf"));
        cx.spawn(async move |this, cx| {
            let path = match chosen.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return log::info!("mail: exporting contacts was cancelled"),
                Ok(Err(error)) => return log::error!("mail: the export panel failed: {error}"),
                Err(error) => return log::error!("mail: the export panel went away: {error}"),
            };
            this.update(cx, |mail, cx| mail.export_to(path, cx)).ok();
        })
        .detach();
    }

    fn export_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let target = path.clone();
        let work = move |store: &mut ely_mail_engine::Store| {
            let cards = store.export_contacts()?;
            ely_files::export(&target, None, cards.as_bytes(), 0).map_err(|err| Error::Store(err.to_string()))?;
            Ok(())
        };
        self.with_store(cx, work, move |mail, written, cx| match written {
            Ok(()) => mail.saved_file(path, cx),
            Err(error) => mail.failed("mail.contacts.unexported", &error, cx),
        });
    }

    pub fn contacts_page(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(people) = &self.people else {
            return div().into_any_element();
        };
        let list = match &people.list {
            None => return div().into_any_element(),
            Some(Err(error)) => {
                let words = text(cx, "mail.contacts.unread", &[("error", error)]);
                return Alert::new("mail-contacts-unread", Severity::Danger, words).into_any_element();
            }
            Some(Ok(list)) => list,
        };
        let read = people.read;
        let field = |key: &'static str, label: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            window.use_keyed_state((key, read), cx, |window, cx| {
                let hint = text(cx, label, &[]);
                TextInput::new(window, cx).placeholder(hint.clone()).label(hint)
            })
        };
        let name = field("mail-contact-name", "mail.contacts.name", window, cx);
        let address = field("mail-contact-address", "mail.contacts.address", window, cx);
        let this = cx.weak_entity();
        let (typed_name, typed_address) = (name.clone(), address.clone());
        let add = Button::new("mail-contact-add", text(cx, "mail.contacts.add", &[]))
            .size(ControlSize::Sm)
            .on_click({
                let this = this.clone();
                move |_, _, cx| {
                    let name = typed_name.read(cx).text().to_owned();
                    let address = typed_address.read(cx).text().to_owned();
                    this.update(cx, |mail, cx| mail.add_contact(name, address, cx)).ok();
                }
            });
        let button = |id: &'static str, key: &str, act: fn(&mut Mail, &mut Context<Mail>), cx: &mut Context<Self>| {
            let this = this.clone();
            Button::new(id, text(cx, key, &[]))
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    this.update(cx, act).ok();
                })
        };
        let import = button("mail-contacts-import", "mail.contacts.import", Mail::import_contacts, cx);
        let export = button("mail-contacts-export", "mail.contacts.export", Mail::export_contacts, cx);
        let controls = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(div().debug_selector(|| "mail-contact-name".into()).w_48().child(Input::new(&name)))
            .child(div().debug_selector(|| "mail-contact-address".into()).w_64().child(Input::new(&address)))
            .child(div().debug_selector(|| "mail-contact-add".into()).child(add))
            .child(div().flex_1())
            .child(div().debug_selector(|| "mail-contacts-import".into()).child(import))
            .child(div().debug_selector(|| "mail-contacts-export".into()).child(export));
        let rows: AnyElement = match list.is_empty() {
            true => {
                let words = text(cx, "mail.contacts.empty", &[]);
                EmptyState::new("mail-contacts-none", IconName::Users, words).into_any_element()
            }
            false => self.contact_rows(Rc::new(list.clone()), cx),
        };
        div()
            .debug_selector(|| "mail-contacts".into())
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(controls)
            .child(div().flex_1().min_h_0().child(rows))
            .into_any_element()
    }

    /// Only the rows in view are drawn.
    fn contact_rows(&self, list: Rc<Vec<Contact>>, cx: &mut Context<Self>) -> AnyElement {
        let (muted, remove) = (cx.theme().colors.fg_muted, text(cx, "mail.contacts.remove", &[]));
        let this = cx.weak_entity();
        uniform_list("mail-contact-rows", list.len(), move |range, _, _| {
            range
                .map(|index| {
                    let contact = &list[index];
                    let id = contact.id;
                    let this = this.clone();
                    let drop = IconButton::new(("mail-contact-remove", id.0 as u64), IconName::X)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .tooltip(remove.clone())
                        .on_click(move |_, _, cx| {
                            this.update(cx, |mail, cx| mail.remove_contact(id, cx)).ok();
                        });
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap_2()
                        .py_1()
                        .child(div().flex_1().min_w_0().truncate().children(contact.name.clone()))
                        .child(div().flex_1().min_w_0().truncate().text_color(muted).child(contact.address.clone()))
                        .child(div().debug_selector(move || format!("mail-contact-remove-{}", id.0)).child(drop))
                })
                .collect()
        })
        .size_full()
        .into_any_element()
    }
}
