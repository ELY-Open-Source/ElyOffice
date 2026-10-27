//! The writing view, kept across launches (MARK-09, MARK-12).

use std::{cell::RefCell, io, path::Path, rc::Rc};

use ely_gpui_component::{
    documents::DocumentLook,
    forms::{FontPicker, TextInput},
    overlays::{Close, Dialog},
    primitives::Severity,
    theme::ActiveTheme as _,
};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, AppContext as _, Context, IntoElement as _, ParentElement as _, SharedString,
    Styled as _, Window, actions, div, rems,
};
use serde::{Deserialize, Serialize};

use crate::desk::Desk;

actions!(
    mark,
    [
        FocusMode,
        Typewriter,
        LineNumbers,
        Narrow,
        Medium,
        Wide,
        Sans,
        Serif,
        Mono,
        OtherFont,
        Larger,
        Smaller,
        ActualSize
    ]
);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Width {
    Narrow,
    Medium,
    Wide,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Font {
    Sans,
    Serif,
    Mono,
    /// A system family the user picked.
    Named(String),
}

/// How text shows; the Markdown never changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub focus: bool,
    pub typewriter: bool,
    pub width: Width,
    pub font: Font,
    /// Percent of the app's text size.
    pub size: u32,
    /// Source mode numbers lines; 0.1.10's off key is dropped.
    #[serde(rename = "source_line_numbers", default = "shown")]
    pub line_numbers: bool,
}

fn shown() -> bool {
    true
}

impl Default for Look {
    fn default() -> Self {
        Look {
            focus: false,
            typewriter: false,
            width: Width::Medium,
            font: Font::Sans,
            size: 100,
            line_numbers: true,
        }
    }
}

/// A serif face each system carries.
const SERIF: &str = if cfg!(target_os = "linux") {
    "DejaVu Serif"
} else {
    "Georgia"
};

impl Look {
    /// How the document shows text by this look.
    pub fn document(&self, cx: &App) -> DocumentLook {
        let measure = match self.width {
            Width::Narrow => 36.0,
            Width::Medium => 48.0,
            Width::Wide => 64.0,
        };
        let font = match &self.font {
            Font::Sans => None,
            Font::Serif => Some(SERIF.into()),
            Font::Mono => Some(cx.theme().mono_family.clone()),
            Font::Named(family) => Some(family.clone().into()),
        };
        DocumentLook {
            focus: self.focus,
            typewriter: self.typewriter,
            measure: Some(rems(measure)),
            font,
            scale: self.size as f32 / 100.0,
            line_numbers: self.line_numbers,
        }
    }
}

/// The kept look; none kept is the default.
pub fn load(file: &Path) -> io::Result<Look> {
    match std::fs::read(file) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Look::default()),
        Err(error) => Err(error),
    }
}

/// Sizes step by a tenth, from 70 to 200 percent.
const SIZES: (u32, u32, u32) = (70, 200, 10);

impl Desk {
    /// Changes the look; its file and menus follow.
    pub fn relook(&mut self, change: impl FnOnce(&mut Look), cx: &mut Context<Self>) {
        change(&mut self.look);
        self.look.size = self.look.size.clamp(SIZES.0, SIZES.1);
        log::info!("mark: the look is now {:?}", self.look);
        let json = serde_json::to_vec_pretty(&self.look).expect("a look serializes");
        if let Err(error) = ely_files::save_atomic(&self.look_file, &json, 0) {
            log::error!("mark: the look did not save: {error}");
            let said =
                ely_ui::i18n::text(cx, "office.error.engine", &[("error", &error.to_string())]);
            self.toast(said, Severity::Danger, cx);
        }
        self.apply_look(cx);
        let menus = crate::keys::menus(&self.look, cx);
        ely_ui::set_document_menus(
            ely_platform::AppId::Mark,
            &ely_ui::commands::RENAMING,
            menus,
            cx,
        );
        cx.notify();
    }

    /// Every tab's document takes the look.
    fn apply_look(&mut self, cx: &mut Context<Self>) {
        let look = self.look.document(cx);
        let documents: Vec<_> = self
            .core
            .tabs
            .iter()
            .map(|tab| tab.view.read(cx).document().clone())
            .collect();
        for document in documents {
            document.update(cx, |document, cx| document.set_look(look.clone(), cx));
        }
    }

    /// A new tab's document takes the look once.
    pub fn look_once(&mut self, id: u64, cx: &mut Context<Self>) {
        let look = self.look.document(cx);
        let Some(tab) = self.tab_mut(id).filter(|tab| !tab.extra.looked) else {
            return;
        };
        tab.extra.looked = true;
        let document = tab.view.read(cx).document().clone();
        document.update(cx, |document, cx| document.set_look(look, cx));
    }

    pub fn look_actions(element: gpui::Div, cx: &mut Context<Self>) -> gpui::Div {
        use gpui::InteractiveElement as _;
        let step = SIZES.2;
        element
            .on_action(cx.listener(|desk, _: &FocusMode, _, cx| {
                desk.relook(|look| look.focus = !look.focus, cx)
            }))
            .on_action(cx.listener(|desk, _: &Typewriter, _, cx| {
                desk.relook(|look| look.typewriter = !look.typewriter, cx)
            }))
            .on_action(cx.listener(|desk, _: &LineNumbers, _, cx| {
                desk.relook(|look| look.line_numbers = !look.line_numbers, cx)
            }))
            .on_action(cx.listener(|desk, _: &Narrow, _, cx| {
                desk.relook(|look| look.width = Width::Narrow, cx)
            }))
            .on_action(cx.listener(|desk, _: &Medium, _, cx| {
                desk.relook(|look| look.width = Width::Medium, cx)
            }))
            .on_action(
                cx.listener(|desk, _: &Wide, _, cx| {
                    desk.relook(|look| look.width = Width::Wide, cx)
                }),
            )
            .on_action(
                cx.listener(|desk, _: &Sans, _, cx| desk.relook(|look| look.font = Font::Sans, cx)),
            )
            .on_action(
                cx.listener(|desk, _: &Serif, _, cx| {
                    desk.relook(|look| look.font = Font::Serif, cx)
                }),
            )
            .on_action(
                cx.listener(|desk, _: &Mono, _, cx| desk.relook(|look| look.font = Font::Mono, cx)),
            )
            .on_action(cx.listener(|desk, _: &OtherFont, window, cx| {
                desk.font_picking = Some(cx.new(|cx| TextInput::new(window, cx)));
                cx.notify();
            }))
            .on_action(
                cx.listener(move |desk, _: &Larger, _, cx| {
                    desk.relook(|look| look.size += step, cx)
                }),
            )
            .on_action(cx.listener(move |desk, _: &Smaller, _, cx| {
                desk.relook(|look| look.size = look.size.saturating_sub(step), cx)
            }))
            .on_action(
                cx.listener(|desk, _: &ActualSize, _, cx| desk.relook(|look| look.size = 100, cx)),
            )
    }
}

impl Desk {

    /// The system's families, the one in use chosen.
    pub fn font_dialog(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let field = self.font_picking.as_ref()?;
        let this = cx.entity();
        // Only actions get the closer, which hands focus back.
        let closer: Rc<RefCell<Option<Close>>> = Rc::default();
        let mut picker = FontPicker::new("mark-font-picker", field).on_change({
            let (this, closer) = (this.clone(), closer.clone());
            move |family: &SharedString, window, cx| {
                let close = closer.borrow().clone().expect("the dialog gave its closer");
                close(window, cx);
                let font = Font::Named(family.to_string());
                this.update(cx, |desk, cx| desk.relook(|look| look.font = font, cx));
            }
        });
        if let Font::Named(family) = &self.look.font {
            picker = picker.selected(family.clone());
        }
        let close = move |_: &mut Window, cx: &mut App| {
            this.update(cx, |desk, cx| {
                desk.font_picking = None;
                cx.notify();
            })
        };
        let title = ely_ui::i18n::text(cx, "mark.look.other.title", &[]);
        let cancel = ely_office::cancel(cx);
        let dialog = Dialog::new("mark-font-dialog", title, close)
            .child(div().w_full().child(picker))
            .action(move |close| {
                closer.replace(Some(close.clone()));
                cancel(close)
            });
        Some(dialog.into_any_element())
    }
}

#[cfg(test)]
pub(crate) mod font_tests;

#[cfg(test)]
mod tests {
    use ely_gpui_component::documents::DocumentLook;
    use gpui::{
        Entity, Modifiers, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point,
        rems,
    };

    use crate::desk::{
        Desk,
        tests::{open, start},
    };

    fn document_look(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> DocumentLook {
        desk.read_with(cx, |desk, cx| {
            let view = desk.active_view().expect("a tab");
            view.read(cx).document().read(cx).look().clone()
        })
    }

    #[gpui::test]
    fn a_new_tab_takes_the_kept_look(cx: &mut TestAppContext) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let kept = r#"{"focus":true,"typewriter":true,"width":"narrow","font":"serif","size":120}"#;
        std::fs::write(data.path().join("look.json"), kept).expect("a look");
        let path = files.path().join("a.md");
        std::fs::write(
            &path, "a
",
        )
        .expect("a file");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &path, cx);
        let look = document_look(&desk, cx);
        assert!(look.focus && look.typewriter);
        assert_eq!((look.measure, look.scale), (Some(rems(36.0)), 1.2));
        assert_eq!(look.font.as_deref(), Some(super::SERIF));
    }

    #[gpui::test]
    fn a_menu_change_reaches_open_tabs_and_is_kept(cx: &mut TestAppContext) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let path = files.path().join("a.md");
        std::fs::write(
            &path, "a
",
        )
        .expect("a file");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &path, cx);
        let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
        cx.update(|window, cx| window.focus(&focus, cx));
        cx.dispatch_action(super::Wide);
        cx.dispatch_action(super::Larger);
        let look = document_look(&desk, cx);
        assert_eq!((look.measure, look.scale), (Some(rems(64.0)), 1.1));
        let kept = std::fs::read_to_string(data.path().join("look.json")).expect("kept");
        assert!(kept.contains("\"wide\"") && kept.contains("110"), "{kept}");
        assert!(document_look(&desk, cx).line_numbers, "on by default");
        cx.dispatch_action(super::LineNumbers);
        assert!(!document_look(&desk, cx).line_numbers, "the menu hides them");
        let kept = std::fs::read_to_string(data.path().join("look.json")).expect("kept");
        assert!(kept.contains("\"source_line_numbers\": false"), "{kept}");
    }

    #[test]
    fn a_kept_look_reads_back_and_none_is_default() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("look.json");
        assert_eq!(
            super::load(&file).expect("none kept"),
            super::Look::default()
        );
        std::fs::write(
            &file,
            r#"{"focus":true,"typewriter":false,"width":"wide","font":"serif","size":120}"#,
        )
        .expect("a look");
        let look = super::load(&file).expect("it reads");
        assert!(look.focus);
        assert!(look.line_numbers, "looks kept before numbers show them");
        assert_eq!(
            (look.width, look.font, look.size),
            (super::Width::Wide, super::Font::Serif, 120)
        );
    }

    #[test]
    fn numbers_hidden_by_0_1_10_show_again() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("look.json");
        let kept = r#"{"focus":false,"typewriter":false,"width":"medium","font":"sans","size":100,"line_numbers":false}"#;
        std::fs::write(&file, kept).expect("a look");
        assert!(super::load(&file).expect("it reads").line_numbers);
        let hidden = kept.replace("\"line_numbers\":false", "\"source_line_numbers\":false");
        std::fs::write(&file, hidden).expect("a look");
        assert!(!super::load(&file).expect("it reads").line_numbers);
    }

    #[gpui::test]
    fn ctrl_and_the_wheel_size_the_text(cx: &mut TestAppContext) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let path = files.path().join("a.md");
        std::fs::write(
            &path, "a
",
        )
        .expect("a file");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &path, cx);
        let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
        cx.update(|window, cx| window.focus(&focus, cx));
        let at = cx.update(|window, _| window.viewport_size()).center();
        let wheel = |modifiers, y| ScrollEvent {
            position: at,
            delta: ScrollDelta::Lines(point(0.0, y)),
            modifiers,
            ..Default::default()
        };
        let other = match cfg!(target_os = "macos") {
            true => Modifiers {
                control: true,
                ..Default::default()
            },
            false => Modifiers {
                platform: true,
                ..Default::default()
            },
        };
        cx.simulate_event(wheel(other, 3.0));
        cx.run_until_parked();
        assert_eq!(
            document_look(&desk, cx).scale,
            1.0,
            "only Ctrl, or Cmd on macOS"
        );
        let secondary = cx.update(|_, cx| Modifiers::secondary_key(cx));
        cx.simulate_event(wheel(secondary, 3.0));
        cx.run_until_parked();
        assert_eq!(document_look(&desk, cx).scale, 1.1, "a notch is one step");
        cx.simulate_event(wheel(secondary, -1.0));
        cx.simulate_event(wheel(secondary, -1.0));
        cx.run_until_parked();
        assert_eq!(document_look(&desk, cx).scale, 0.9);
    }
}
