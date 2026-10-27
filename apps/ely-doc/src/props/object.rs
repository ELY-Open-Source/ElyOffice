//! The object group: size, crop, replace, anchor, wrap, order (DOC-08).

use ely_gpui_component::{
    buttons::Button, data_display::PropertyGroup, forms::NumberInput, theme::ControlSize,
};
use ely_office::panel::{Panel as _, button, muted, row, toggle};
use gpui::{AnyElement, Context, IntoElement, ParentElement as _, SharedString, Styled as _, div};

use super::Props;
use super::sheet::size_of;
use crate::docs::Docs;

/// 1/100 mm in whole twips, rounded as the kernel rounds.
fn twips_of(mm100: f64) -> i64 {
    ((mm100 * 72.0 + 63.0) / 127.0).floor() as i64
}

impl Props {
    pub(super) fn object_group(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let tip = |key: &str| Docs::text(cx, key, &[]);
        let why = "doc.why.selection";
        let anchors = row([
            toggle(
                self,
                "image.anchor_page",
                (None, &tip("doc.props.to_page")),
                ("", "doc.why.anchor"),
                cx,
            ),
            toggle(
                self,
                "image.anchor_paragraph",
                (None, &tip("doc.props.to_paragraph")),
                ("", "doc.why.anchor"),
                cx,
            ),
            toggle(
                self,
                "image.anchor_char",
                (None, &tip("doc.props.to_char")),
                ("", "doc.why.anchor"),
                cx,
            ),
            toggle(
                self,
                "image.anchor_as_char",
                (None, &tip("doc.props.as_char")),
                ("", "doc.why.anchor"),
                cx,
            ),
        ]);
        let wraps = row([
            toggle(
                self,
                "image.wrap_off",
                (None, &tip("doc.props.wrap_none")),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "image.wrap_on",
                (None, &tip("doc.props.wrap_both")),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "image.wrap_ideal",
                (None, &tip("doc.props.wrap_ideal")),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "image.wrap_left",
                (None, &tip("doc.props.wrap_left")),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "image.wrap_right",
                (None, &tip("doc.props.wrap_right")),
                ("", why),
                cx,
            ),
            toggle(
                self,
                "image.wrap_through",
                (None, &tip("doc.props.wrap_through")),
                ("", why),
                cx,
            ),
        ]);
        let order = row([
            button(self, "image.front", "doc.props.front", why, cx),
            button(self, "image.forward", "doc.props.forward", why, cx),
            button(self, "image.backward", "doc.props.backward", why, cx),
            button(self, "image.back", "doc.props.back", why, cx),
        ]);
        let replace = Button::new("doc-props-replace", tip("doc.props.replace"))
            .size(ControlSize::Sm)
            .on_click(|_, window, cx| {
                window.dispatch_action(Box::new(crate::insert::InsertImage), cx)
            });
        PropertyGroup::new(tip("doc.props.object"))
            .row(tip("doc.props.width"), self.extent(false, cx))
            .row(tip("doc.props.height"), self.extent(true, cx))
            .row(
                tip("doc.props.image"),
                row([
                    button(self, "image.crop", "doc.props.crop", why, cx),
                    replace.into_any_element(),
                ]),
            )
            .row(tip("doc.props.anchor"), anchors)
            .row(tip("doc.props.wrap"), wraps)
            .row(tip("doc.props.order"), order)
    }

    /// Sizes the image from one side in cm.
    pub(super) fn set_extent(&mut self, tall: bool, cm: f64, cx: &mut Context<Self>) {
        let Some((width, height)) = self.state(".uno:Size", cx).as_deref().and_then(size_of) else {
            return;
        };
        let mine = if tall { height } else { width };
        let scale = cm * 1000.0 / f64::from(mine);
        let (w, h) = (f64::from(width) * scale, f64::from(height) * scale);
        let unchanged =
            twips_of(w) == twips_of(f64::from(width)) && twips_of(h) == twips_of(f64::from(height));
        if unchanged {
            return;
        }
        let twips = [twips_of(w).to_string(), twips_of(h).to_string()];
        self.run("image.size", &[&twips[0], &twips[1]], cx);
    }

    /// Width or height in cm, ratio kept.
    fn extent(&self, tall: bool, cx: &Context<Self>) -> AnyElement {
        let Some((width, height)) = self.state(".uno:Size", cx).as_deref().and_then(size_of) else {
            return muted("—".into(), cx);
        };
        if width <= 0 || height <= 0 {
            return muted("—".into(), cx);
        }
        let this = cx.entity();
        let mine = if tall { height } else { width };
        let id = SharedString::from(format!("doc-props-extent-{tall}"));
        let field = NumberInput::new(id, f64::from(mine) / 1000.0)
            .label(Docs::text(cx, if tall { "doc.props.height" } else { "doc.props.width" }, &[]))
            .range(0.1, 600.0)
            .step(0.1)
            .precision(2)
            .size(ControlSize::Sm)
            .on_commit(move |cm, _, cx| {
                this.update(cx, |props, cx| props.set_extent(tall, cm, cx))
            });
        div().w_full().max_w_32().child(field).into_any_element()
    }
}
