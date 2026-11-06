//! A body's right-press menu: its text, link and picture (#68).

use ely_gpui_component::{
    documents::{HeldPicture, RightPress},
    menus::{ContextMenu, Menu, MenuItem},
    primitives::IconName,
};
use ely_mail_engine::MessageId;
use ely_platform::clipboard::{self, Clip};
use ely_ui::menu::Offer;
use gpui::{ClipboardItem, Context, ImageFormat, SharedString};

use crate::mail::{Mail, text};

/// A right press in one message's body.
pub struct Asked {
    pub id: MessageId,
    pub press: RightPress,
}

/// A file name for the picture at `url`.
fn picture_name(url: &str, format: ImageFormat, unnamed: &str) -> String {
    // Inline pictures carry no name in their address.
    let tail = match url.starts_with("http://") || url.starts_with("https://") {
        true => url.rsplit('/').next().unwrap_or_default(),
        false => "",
    };
    let tail = tail.split(['?', '#']).next().unwrap_or_default();
    // The bytes' format names the type, not the address.
    let stem = tail.rsplit_once('.').map_or(tail, |(stem, _)| stem);
    let stem = if stem.is_empty() { unnamed } else { stem };
    let name = format!("{stem}.{}", format.extension());
    crate::migrate::file_name_of(&name, unnamed)
}

impl Mail {
    /// A menu where the press fell, if it found anything.
    pub(crate) fn body_pressed(&mut self, id: MessageId, press: RightPress, cx: &mut Context<Self>) {
        if press.selected.is_none() && press.link.is_none() && press.image.is_none() {
            return log::info!("mail: a right press in a body found nothing to offer");
        }
        let at = press.position;
        self.body_asked = Some(Offer::new(self.body_asked.as_ref(), 0, Asked { id, press }, at));
        cx.notify();
    }

    /// The menu's host; it stays, so focus survives the menu.
    pub(crate) fn body_menu_host(&self, cx: &mut Context<Self>) -> ContextMenu {
        match &self.body_asked {
            Some(offer) => offer.host("mail-body-menu", self.body_menu(&offer.what, cx)),
            None => ContextMenu::new("mail-body-menu", Menu::new()).manual(None),
        }
    }

    fn body_menu(&self, asked: &Asked, cx: &mut Context<Self>) -> Menu {
        let (mut menu, mut offered) = (Menu::new(), false);
        for (key, icon, copied) in [
            ("mail.body.copy", IconName::Copy, &asked.press.selected),
            ("mail.body.copy_link", IconName::Link, &asked.press.link),
        ] {
            let Some(copied) = copied.clone() else {
                continue;
            };
            let item = MenuItem::new(text(cx, key, &[])).icon(icon).on_click(move |_, cx| {
                log::info!("mail: {key} from a body");
                cx.write_to_clipboard(ClipboardItem::new_string(copied.clone()));
            });
            (menu, offered) = (menu.item(item), true);
        }
        let Some(url) = asked.press.image.clone() else {
            return menu;
        };
        let copy = MenuItem::new(text(cx, "mail.body.copy_picture", &[])).icon(IconName::Image);
        let save = MenuItem::new(text(cx, "mail.body.save_picture", &[])).icon(IconName::Download);
        let held = match self.held_picture(asked.id, &url) {
            Some(Ok(held)) => held,
            Some(Err(why)) => return menu.group(why, [copy.disabled(true), save.disabled(true)]),
            None => {
                let why = text(cx, "mail.body.blocked", &[]);
                return menu.group(why, [copy.disabled(true), save.disabled(true)]);
            }
        };
        let (this, kept) = (cx.weak_entity(), held.clone());
        let copy = copy.on_click(move |_, cx| {
            this.update(cx, |mail, cx| mail.copy_picture(&kept, cx)).ok();
        });
        let this = cx.weak_entity();
        let save = save.on_click(move |_, cx| {
            this.update(cx, |mail, cx| mail.save_picture(&held, &url, cx)).ok();
        });
        if offered {
            menu = menu.separator();
        }
        menu.item(copy).item(save)
    }

    /// The picture `url` names in message `id`.
    fn held_picture(&self, id: MessageId, url: &str) -> Option<Result<HeldPicture, SharedString>> {
        let thread = self.reading.shown.as_ref()?.thread.as_ref()?.as_ref().ok()?;
        let each = thread.letters.iter().find(|each| each.stored.id == id)?;
        each.open.as_ref().ok()?.pictures.get(url).cloned()
    }

    fn copy_picture(&mut self, held: &HeldPicture, cx: &mut Context<Self>) {
        match clipboard::write(&Clip::Png(held.bytes().to_vec())) {
            Ok(()) => log::info!("mail: a picture of {} bytes was copied", held.bytes().len()),
            Err(error) => self.failed("mail.body.uncopied", &format!("{error:#}"), cx),
        }
    }

    fn save_picture(&mut self, held: &HeldPicture, url: &str, cx: &mut Context<Self>) {
        let unnamed = text(cx, "mail.body.picture_name", &[]).to_string();
        let name = picture_name(url, held.format(), &unnamed);
        self.save_bytes(name, held.bytes().clone(), "mail.body.unsaved", cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui::ImageFormat;

    use super::picture_name;

    #[test]
    fn a_picture_takes_its_name_from_its_address() {
        let named = |url| picture_name(url, ImageFormat::Png, "picture");
        assert_eq!(named("https://example.com/a/cat.png?size=2"), "cat.png");
        assert_eq!(named("https://example.com/photo.jpg"), "photo.png", "a PNG served as .jpg");
        assert_eq!(named("https://example.com/logo#top"), "logo.png");
        assert_eq!(named("ely-inline:0"), "picture.png");
        assert_eq!(named("https://example.com/"), "picture.png");
    }
}
