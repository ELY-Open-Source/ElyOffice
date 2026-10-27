//! The panel's text group and its kernel hookup.

use ely_gpui_component::{data_display::PropertyGroup, primitives::IconName};
use ely_office::panel::{Panel, color_state, font_box, palette, row, size_menu, toggle};
use gpui::{App, Context, InteractiveElement, ParentElement, SharedString, Window, div};

use super::Props;
use crate::docs::Docs;

/// The swatch's label without a color.
const AUTOMATIC: &str = "office.props.automatic";

impl Panel for Props {
    fn state(&self, uno: &str, cx: &App) -> Option<SharedString> {
        let view = self.shown.as_ref()?.view.read(cx);
        view.state(uno)
            .map(|state| SharedString::from(state.to_string()))
    }

    fn run(&mut self, id: &str, values: &[&str], cx: &mut Context<Self>) {
        Props::run(self, id, values, cx);
    }
}

impl Props {
    pub(super) fn text_group(&self, window: &mut Window, cx: &mut Context<Self>) -> PropertyGroup {
        let why = "doc.why.selection";
        let effects = row([
            toggle(
                self,
                "format.bold",
                (Some(IconName::Bold), ""),
                ("doc.props.bold", why),
                cx,
            ),
            toggle(
                self,
                "format.italic",
                (Some(IconName::Italic), ""),
                ("doc.props.italic", why),
                cx,
            ),
            toggle(
                self,
                "format.underline",
                (Some(IconName::Underline), ""),
                ("doc.props.underline", why),
                cx,
            ),
            toggle(
                self,
                "format.strikeout",
                (Some(IconName::Strikethrough), ""),
                ("doc.props.strikeout", why),
                cx,
            ),
            toggle(
                self,
                "format.superscript",
                (None, "x²"),
                ("doc.props.superscript", why),
                cx,
            ),
            toggle(
                self,
                "format.subscript",
                (None, "x₂"),
                ("doc.props.subscript", why),
                cx,
            ),
        ]);
        let this = cx.entity();
        let color = color_state(self, ".uno:FontColor", cx);
        let highlight = color_state(self, ".uno:CharBackColor", cx);
        let name = Docs::text(cx, "doc.props.font", &[]);
        let font = font_box(self, ("doc-props-font", name), &self.fonts, window, cx);
        let tip = |key: &str| Docs::text(cx, key, &[]);
        PropertyGroup::new(tip("doc.props.text"))
            .row(tip("doc.props.font"), font)
            .row(tip("doc.props.size"), size_menu(self, ("doc-props-size", tip("doc.props.size")), cx))
            .row(
                tip("doc.props.effects"),
                div()
                    .debug_selector(|| "doc-props-text".into())
                    .child(effects),
            )
            .row(
                tip("doc.props.color"),
                palette(&this, "format.color", (color, AUTOMATIC), None, cx),
            )
            .row(
                tip("doc.props.highlight"),
                palette(
                    &this,
                    "format.highlight",
                    (highlight, AUTOMATIC),
                    Some(("doc.props.no_highlight", "format.highlight", "-1")),
                    cx,
                ),
            )
    }
}
