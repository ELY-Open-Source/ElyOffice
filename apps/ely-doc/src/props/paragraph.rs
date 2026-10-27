//! The paragraph group: alignment, spacing, lists, tab stops (DOC-03).

use ely_gpui_component::{
    buttons::Button, data_display::PropertyGroup, forms::NumberInput, primitives::IconName,
    theme::ControlSize,
};
use ely_office::panel::{Panel as _, button, muted, row, send, toggle};
use gpui::{AnyElement, Context, IntoElement, ParentElement as _, Styled as _, div};

use super::Props;
use crate::docs::Docs;

/// The kernel's spacing state: "upper,lower" in 1/100 mm.
pub(super) const SPACING: &str = ".uno:ULSpacing";

/// Twips in one centimeter.
const TWIPS_PER_CM: f64 = 567.0;

impl Props {
    pub(super) fn paragraph_group(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let tip = |key: &str| Docs::text(cx, key, &[]);
        let align = row([
            toggle(
                self,
                "paragraph.left",
                (Some(IconName::AlignLeft), ""),
                ("doc.props.left", "doc.why.selection"),
                cx,
            ),
            toggle(
                self,
                "paragraph.center",
                (Some(IconName::AlignCenter), ""),
                ("doc.props.center", "doc.why.selection"),
                cx,
            ),
            toggle(
                self,
                "paragraph.right",
                (Some(IconName::AlignRight), ""),
                ("doc.props.right", "doc.why.selection"),
                cx,
            ),
            toggle(
                self,
                "paragraph.justify",
                (Some(IconName::AlignJustify), ""),
                ("doc.props.justify", "doc.why.selection"),
                cx,
            ),
        ]);
        let line = row([
            toggle(
                self,
                "paragraph.spacing_1",
                (None, "1"),
                ("", "doc.why.selection"),
                cx,
            ),
            toggle(
                self,
                "paragraph.spacing_15",
                (None, "1.5"),
                ("", "doc.why.selection"),
                cx,
            ),
            toggle(
                self,
                "paragraph.spacing_2",
                (None, "2"),
                ("", "doc.why.selection"),
                cx,
            ),
        ]);
        let lists = row([
            toggle(
                self,
                "list.bullets",
                (Some(IconName::List), ""),
                ("doc.props.bullets", "doc.why.selection"),
                cx,
            ),
            toggle(
                self,
                "list.numbering",
                (Some(IconName::ListOrdered), ""),
                ("doc.props.numbering", "doc.why.selection"),
                cx,
            ),
            button(
                self,
                "list.higher",
                "doc.props.promote",
                "doc.why.selection",
                cx,
            ),
            button(
                self,
                "list.deeper",
                "doc.props.demote",
                "doc.why.selection",
                cx,
            ),
        ]);
        PropertyGroup::new(tip("doc.props.paragraph"))
            .row(tip("doc.props.align"), align)
            .row(
                tip("doc.props.indent"),
                row([
                    button(
                        self,
                        "paragraph.outdent",
                        "doc.props.less",
                        "doc.why.selection",
                        cx,
                    ),
                    button(
                        self,
                        "paragraph.indent",
                        "doc.props.more",
                        "doc.why.selection",
                        cx,
                    ),
                ]),
            )
            .row(tip("doc.props.line"), line)
            .row(tip("doc.props.above"), self.spacing(false, cx))
            .row(tip("doc.props.below"), self.spacing(true, cx))
            .row(tip("doc.props.lists"), lists)
            .row(tip("doc.props.tabs"), self.tabs(cx))
    }

    /// Space above or below, in points.
    fn spacing(&self, below: bool, cx: &Context<Self>) -> AnyElement {
        if self.state(SPACING, cx).as_deref() == Some(ely_lok::MIXED) {
            return muted(Docs::text(cx, "office.props.mixed", &[]), cx);
        }
        let Some((upper, lower)) = self.pair(SPACING, cx) else {
            return muted("—".into(), cx);
        };
        let this = cx.entity();
        let (id, mine) = match below {
            true => ("doc-props-below", lower),
            false => ("doc-props-above", upper),
        };
        let field = NumberInput::new(id, f64::from(mine) * 72.0 / 2540.0)
            .label(Docs::text(cx, if below { "doc.props.below" } else { "doc.props.above" }, &[]))
            .range(0.0, 283.0)
            .step(1.0)
            .precision(1)
            .size(ControlSize::Sm)
            .on_commit(move |points, _, cx| {
                let twips = (points * 20.0).round() as i32;
                this.update(cx, |props, cx| {
                    props.set_pair((SPACING, "paragraph.spacing"), below, twips, cx)
                })
            });
        div().w_full().max_w_32().child(field).into_any_element()
    }

    /// The paragraph's stops, each removable, and an adder in cm.
    fn tabs(&self, cx: &Context<Self>) -> AnyElement {
        let this = cx.entity();
        let stops = self
            .shown
            .as_ref()
            .map(|shown| shown.view.read(cx).tabs().to_vec())
            .unwrap_or_default();
        let chips: Vec<AnyElement> = stops
            .into_iter()
            .enumerate()
            .map(|(at, mm100)| {
                let this = this.clone();
                let cm = format!("{:.2}", f64::from(mm100) / 1000.0);
                let label = Docs::text(cx, "doc.props.cm", &[("n", &cm)]);
                Button::new(("doc-props-tab", at), label)
                    .size(ControlSize::Sm)
                    .trailing_icon(IconName::X)
                    .on_click(move |_, _, cx| {
                        send(&this, "paragraph.tab", &[&at.to_string(), "0", "true"], cx)
                    })
                    .into_any_element()
            })
            .collect();
        let at = NumberInput::new("doc-props-tab-at", self.tab_at)
            .label(Docs::text(cx, "doc.props.tabs", &[]))
            .range(0.0, 55.0)
            .step(0.25)
            .precision(2)
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| this.update(cx, |props, _| props.tab_at = value)
            });
        let add = Button::new(
            "doc-props-tab-add",
            Docs::text(cx, "doc.props.add_tab", &[]),
        )
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| {
            let twips = this.read(cx).tab_at * TWIPS_PER_CM;
            let twips = (twips.round() as i64).to_string();
            send(&this, "paragraph.tab", &["-1", &twips, "false"], cx)
        });
        let at = div().w_full().max_w_32().child(at).into_any_element();
        row(chips.into_iter().chain([at, add.into_any_element()]))
    }
}
