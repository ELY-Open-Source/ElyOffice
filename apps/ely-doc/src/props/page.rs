//! The page group: paper, margins, columns, sections, headers (DOC-05, 06).

use ely_gpui_component::{
    buttons::{ToggleButton, ToggleItem},
    data_display::PropertyGroup,
    forms::{Choice, NumberInput, Select},
    theme::ControlSize,
};
use ely_office::panel::{Panel as _, known, muted, row, send};
use gpui::{AnyElement, Context, IntoElement, ParentElement as _, SharedString, Styled as _, div};

use super::Props;
use super::builtin::{self, PAGE};
use super::sheet::Sheet;
#[cfg(test)]
use super::sheet::{page_of, size_of};
use crate::docs::Docs;

/// Paper sizes offered, portrait, in 1/100 mm.
const PAPERS: [(&str, i32, i32); 6] = [
    ("A3", 29_700, 42_000),
    ("A4", 21_000, 29_700),
    ("A5", 14_800, 21_000),
    ("B5", 17_600, 25_000),
    ("Letter", 21_590, 27_940),
    ("Legal", 21_590, 35_560),
];

const LEFT_RIGHT: (&str, &str) = (".uno:PageLRMargin", "page.margins_lr");

/// A header or footer: switch and layout.
struct Part {
    on: (&'static str, &'static str),
    layout: (&'static str, &'static str),
    why: &'static str,
}

const HEADER: Part = Part {
    on: ("header.on", ".uno:SwPageHeaderItem"),
    layout: ("header.layout", ".uno:SwPageHeaderLayout"),
    why: "doc.why.header",
};

const FOOTER: Part = Part {
    on: ("footer.on", ".uno:SwPageFooterItem"),
    layout: ("footer.layout", ".uno:SwPageFooterLayout"),
    why: "doc.why.footer",
};
const TOP_BOTTOM: (&str, &str) = (".uno:PageULMargin", "page.margins_ul");

/// The offered paper within the kernel's rounding of `size`.
fn paper_of((width, height): (i32, i32)) -> Option<&'static str> {
    let (short, long) = (width.min(height), width.max(height));
    PAPERS
        .iter()
        .find(|(_, w, h)| (short - w).abs() <= 2 && (long - h).abs() <= 2)
        .map(|(name, ..)| *name)
}

impl Props {
    pub(super) fn page_group(&self, cx: &mut Context<Self>) -> PropertyGroup {
        let tip = |key: &str| Docs::text(cx, key, &[]);
        PropertyGroup::new(tip("doc.props.page"))
            .row(tip("doc.props.paper"), self.paper(cx))
            .row(tip("doc.props.orientation"), self.orientation(cx))
            .row(tip("doc.props.top"), self.margin(TOP_BOTTOM, false, cx))
            .row(tip("doc.props.bottom"), self.margin(TOP_BOTTOM, true, cx))
            .row(
                tip("doc.props.margin_left"),
                self.margin(LEFT_RIGHT, false, cx),
            )
            .row(
                tip("doc.props.margin_right"),
                self.margin(LEFT_RIGHT, true, cx),
            )
            .row(tip("doc.props.columns"), self.columns(cx))
            .row(tip("doc.props.section"), self.section(cx))
            .row(tip("doc.props.header"), self.part(HEADER, cx))
            .row(tip("doc.props.footer"), self.part(FOOTER, cx))
    }

    fn paper(&self, cx: &Context<Self>) -> AnyElement {
        let Some(Sheet { size, .. }) = self.sheet(cx) else {
            return muted("—".into(), cx);
        };
        let this = cx.entity();
        let choices = PAPERS.iter().map(|(name, ..)| {
            let key = format!("doc.paper.{}", name.to_ascii_lowercase());
            Choice::new(*name, Docs::text(cx, &key, &[]))
        });
        let cm = |mm100: i32| format!("{:.2}", f64::from(mm100) / 1000.0);
        let custom = Docs::text(
            cx,
            "doc.props.size_cm",
            &[("w", &cm(size.0)), ("h", &cm(size.1))],
        );
        let mut menu = Select::new("doc-props-paper", choices)
            .size(ControlSize::Sm)
            .placeholder(custom)
            .on_change(move |name, _, cx| this.update(cx, |props, cx| props.set_paper(name, cx)));
        if let Some(name) = paper_of(size) {
            menu = menu.selected(name);
        }
        menu.into_any_element()
    }

    fn orientation(&self, cx: &Context<Self>) -> AnyElement {
        let Some(Sheet { landscape, .. }) = self.sheet(cx) else {
            return muted("—".into(), cx);
        };
        let face = |wide: bool| {
            let this = cx.entity();
            let key = if wide {
                "doc.props.landscape"
            } else {
                "doc.props.portrait"
            };
            let item = ToggleItem::new(key).label(Docs::text(cx, key, &[]));
            ToggleButton::new(SharedString::from(key), item, landscape == wide)
                .size(ControlSize::Sm)
                .on_toggle(move |_, _, cx| {
                    this.update(cx, |props, cx| props.set_orientation(wide, cx))
                })
                .into_any_element()
        };
        row([face(false), face(true)])
    }

    /// An offered paper, in the page's orientation.
    pub(super) fn set_paper(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(sheet) = self.sheet(cx) else {
            return;
        };
        let (_, w, h) = PAPERS
            .iter()
            .find(|(paper, ..)| *paper == name)
            .expect("an offered paper");
        let size = if sheet.landscape { (*h, *w) } else { (*w, *h) };
        self.send_sheet(Sheet { size, ..sheet }, cx);
    }

    /// Turns the page, size and flag together.
    pub(super) fn set_orientation(&mut self, wide: bool, cx: &mut Context<Self>) {
        let Some(sheet) = self.sheet(cx) else {
            return;
        };
        let (width, height) = sheet.size;
        let (short, long) = (width.min(height), width.max(height));
        let size = if wide { (long, short) } else { (short, long) };
        self.send_sheet(
            Sheet {
                size,
                landscape: wide,
                ..sheet
            },
            cx,
        );
    }

    /// One page margin in cm.
    fn margin(
        &self,
        pair: (&'static str, &'static str),
        second: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        if self.state(pair.0, cx).as_deref() == Some(ely_lok::MIXED) {
            return muted(Docs::text(cx, "office.props.mixed", &[]), cx);
        }
        let Some((first, other)) = self.pair(pair.0, cx) else {
            return muted("—".into(), cx);
        };
        let this = cx.entity();
        let mine = if second { other } else { first };
        let id = SharedString::from(format!("doc-props-{}-{second}", pair.1));
        let field = NumberInput::new(id, f64::from(mine) / 1000.0)
            .range(0.0, 99.99)
            .step(0.1)
            .precision(2)
            .size(ControlSize::Sm)
            .on_commit(move |cm, _, cx| {
                let twips = (cm * 1440.0 / 2.54).round() as i32;
                this.update(cx, |props, cx| props.set_pair(pair, second, twips, cx))
            });
        div().w_full().max_w_32().child(field).into_any_element()
    }

    fn columns(&self, cx: &Context<Self>) -> AnyElement {
        let current = self.state(".uno:PageColumnType", cx);
        row((1..=3).map(|count: i32| {
            let this = cx.entity();
            let label = count.to_string();
            let on = current.as_deref() == Some(label.as_str());
            ToggleButton::new(
                SharedString::from(format!("doc-props-columns-{count}")),
                ToggleItem::new(label.clone()).label(label),
                on,
            )
            .size(ControlSize::Sm)
            .on_toggle(move |_, _, cx| send(&this, "page.columns", &[&count.to_string()], cx))
            .into_any_element()
        }))
    }

    /// The current section's page style.
    fn section(&self, cx: &Context<Self>) -> AnyElement {
        let names = self
            .shown
            .as_ref()
            .map(|shown| shown.pages.clone())
            .unwrap_or_default();
        let current = self
            .state(".uno:PageStyleName", cx)
            .filter(|name| known(name))
            .map(|name| SharedString::from(builtin::listed(&PAGE, &name)));
        let this = cx.entity();
        let empty = names.is_empty();
        let mut menu = Select::new(
            "doc-props-section",
            names.into_iter().map(|name| {
                let label = builtin::shown(&PAGE, &name, cx);
                Choice::new(name, label)
            }),
        )
        .size(ControlSize::Sm)
        .disabled(empty)
        .placeholder(current.as_deref().map(|name| builtin::shown(&PAGE, name, cx)).unwrap_or_default())
        .on_change(move |name, _, cx| {
            send(&this, "format.style", &[name.as_ref(), "PageStyles"], cx)
        });
        if let Some(name) = current {
            menu = menu.selected(name);
        }
        menu.into_any_element()
    }

    /// On or off, then first and odd pages.
    fn part(&self, part: Part, cx: &Context<Self>) -> AnyElement {
        let (on_command, on_uno) = part.on;
        let (layout_command, layout_uno) = part.layout;
        let on = self.state(on_uno, cx).as_deref() == Some("true");
        let switch = {
            let this = cx.entity();
            let key = if on { "doc.props.on" } else { "doc.props.off" };
            let item = ToggleItem::new(on_command).label(Docs::text(cx, key, &[]));
            ToggleButton::new(SharedString::from(on_command), item, on)
                .size(ControlSize::Sm)
                .on_toggle(move |turn_on, _, cx| {
                    send(&this, on_command, &[&turn_on.to_string()], cx)
                })
                .into_any_element()
        };
        let current = self
            .state(layout_uno, cx)
            .and_then(|state| state.parse::<i32>().ok());
        let face = |bit: i32, key: &'static str| {
            let this = cx.entity();
            let mut item = ToggleItem::new(key).label(Docs::text(cx, key, &[]));
            if current.is_none() {
                item = item.tooltip(Docs::text(cx, part.why, &[]));
            }
            let value = current.unwrap_or(0);
            let id = SharedString::from(format!("{layout_command}-{bit}"));
            ToggleButton::new(id, item, value & bit != 0)
                .disabled(current.is_none())
                .size(ControlSize::Sm)
                .on_toggle(move |_, _, cx| {
                    send(&this, layout_command, &[&(value ^ bit).to_string()], cx)
                })
                .into_any_element()
        };
        row([
            switch,
            face(1, "doc.props.first_page"),
            face(2, "doc.props.odd_even"),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_states_read_and_papers_match_either_way() {
        assert_eq!(size_of("21590 x 27940"), Some((21590, 27940)));
        assert_eq!(page_of("true,0,4"), Some((true, 0, 4)));
        assert_eq!(
            paper_of((29700, 21001)),
            Some("A4"),
            "landscape A4 after rounding"
        );
        assert_eq!(paper_of((21590, 27940)), Some("Letter"));
        assert_eq!(paper_of((10000, 10000)), None);
        assert_eq!(size_of("garbage"), None);
    }
}
