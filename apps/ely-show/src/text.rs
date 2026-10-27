//! The text group of the object panel (SHOW-03).

use ely_gpui_component::{
    data_display::PropertyGroup,
    forms::{Choice, Select},
    primitives::IconName,
    theme::ControlSize,
};
use ely_lok::show::Selection;
use ely_office::panel::{self as kernel, font_box, palette, row, size_menu, toggle};
use gpui::{AnyElement, Context, IntoElement, SharedString, Window};

use crate::deck::Deck;
use crate::design::{Panel, on_deck};

impl Panel {
    pub(crate) fn words(
        &self,
        sel: &Selection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PropertyGroup {
        let why = "show.object.why_text";
        let effects = row([
            toggle(
                self,
                "format.bold",
                (Some(IconName::Bold), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "format.italic",
                (Some(IconName::Italic), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "format.underline",
                (Some(IconName::Underline), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "format.strikeout",
                (Some(IconName::Strikethrough), ""),
                ("", why),
                cx,
            ),
        ]);
        let align = row([
            toggle(
                self,
                "paragraph.left",
                (Some(IconName::AlignLeft), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "paragraph.center",
                (Some(IconName::AlignCenter), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "paragraph.right",
                (Some(IconName::AlignRight), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "paragraph.justify",
                (Some(IconName::AlignJustify), ""),
                ("", why),
                cx,
            ),
        ]);
        let lists = row([
            toggle(
                self,
                "list.bullets",
                (Some(IconName::List), ""),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "list.numbering",
                (Some(IconName::ListOrdered), ""),
                ("", why),
                cx,
            ),
        ]);
        let this = cx.entity();
        let ink = kernel::color_state(self, ".uno:FontColor", cx);
        let fonts = self.fonts(cx);
        let name = Deck::text(cx, "show.object.font", &[]);
        let font = font_box(self, ("show-font", name), &fonts, window, cx);
        let size = size_menu(self, ("show-size", Deck::text(cx, "show.object.font_size", &[])), cx);
        let fit = self.fit(&sel.autofit, cx);
        let tip = |key: &str| Deck::text(cx, key, &[]);
        PropertyGroup::new(tip("show.object.text"))
            .row(tip("show.object.font"), font)
            .row(tip("show.object.font_size"), size)
            .row(tip("show.object.effects"), effects)
            .row(
                tip("show.object.color"),
                palette(
                    &this,
                    "format.color",
                    (ink, "office.props.automatic"),
                    None,
                    cx,
                ),
            )
            .row(tip("show.object.align"), align)
            .row(tip("show.object.lists"), lists)
            .row(tip("show.object.autofit"), fit)
    }

    /// Off, shrink on overflow, or grow the box.
    fn fit(&self, now: &str, cx: &mut Context<Self>) -> AnyElement {
        let label = |key: &str| Deck::text(cx, key, &[]);
        let mut choices: Vec<Choice> = ["off", "shrink", "grow"]
            .iter()
            .map(|key| Choice::new(*key, label(&format!("show.fit.{key}"))))
            .collect();
        if now == "mixed" {
            choices.push(Choice::new("mixed", label("office.props.mixed")));
        }
        let deck = self.deck.clone();
        Select::new("show-fit", choices)
            .label(label("show.object.autofit"))
            .selected(SharedString::from(now.to_string()))
            .size(ControlSize::Sm)
            .on_change(move |to, _, cx| {
                let to = to.to_string();
                if to != "mixed" {
                    on_deck(&deck, cx, |deck, cx| {
                        deck.object_command("text.fit", &[&to], cx)
                    })
                }
            })
            .into_any_element()
    }
}
