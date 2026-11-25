//! The cell borders: line style, width, color and edges.

use ely_gpui_component::{
    buttons::Button,
    forms::{Choice, ColorPalette, ColorSwatch, Select},
    overlays::Popover,
    theme::ControlSize,
};
use gpui::{AnyElement, Context, Hsla, IntoElement as _, SharedString, rgb};

use super::{Panel, row, run};
use crate::book::Book;

/// Border line colors, 0xRRGGBB.
const LINES: [u32; 10] = [
    0x000000, 0x595959, 0x7F7F7F, 0xBFBFBF, 0xC00000, 0xED7D31, 0xFFC000, 0x70AD47, 0x4472C4,
    0x7030A0,
];

/// Line styles: kernel number, sample, label key.
const STYLES: [(i64, &str, &str); 7] = [
    (0, "────", "table.format.line_solid"),
    (1, "┈┈┈┈", "table.format.line_dotted"),
    (2, "─ ─ ─", "table.format.line_dashed"),
    (14, "╌╌╌╌", "table.format.line_fine"),
    (16, "─ · ─", "table.format.line_dash_dot"),
    (17, "─ ·· ─", "table.format.line_dash_dot_dot"),
    (3, "════", "table.format.line_double"),
];

/// Border line widths in 1/100 mm and their label keys.
const WIDTHS: [(i64, &str); 5] = [
    (9, "table.format.width_hair"),
    (18, "table.format.width_thin"),
    (35, "table.format.width_medium"),
    (53, "table.format.width_thick"),
    (88, "table.format.width_heavy"),
];

/// A double line shows nothing narrower.
const DOUBLE_MIN: i64 = 35;

/// Marks for L, R, T, B, H, V edges.
const PRESETS: [(&str, &str); 12] = [
    ("111111", "table.format.borders_all"),
    ("111100", "table.format.borders_outer"),
    ("000011", "table.format.borders_inner"),
    ("000010", "table.format.borders_inner_h"),
    ("000001", "table.format.borders_inner_v"),
    ("001000", "table.format.borders_top"),
    ("000100", "table.format.borders_bottom"),
    ("100000", "table.format.borders_left"),
    ("010000", "table.format.borders_right"),
    ("001100", "table.format.borders_rows"),
    ("110000", "table.format.borders_columns"),
    ("xxxxxx", "table.format.borders_none"),
];

/// The line the border buttons draw.
#[derive(Clone, Copy)]
pub(super) struct Line {
    style: i64,
    width: i64,
    color: u32,
}

impl Default for Line {
    fn default() -> Self {
        Line { style: 0, width: 18, color: 0 }
    }
}

impl Line {
    /// The edge value for a preset mark.
    fn edge(self, mark: char) -> String {
        match mark {
            '1' => {
                let width = if self.style == 3 { self.width.max(DOUBLE_MIN) } else { self.width };
                format!("{}:{}:{}", self.style, width, self.color)
            }
            'x' => "none".into(),
            _ => "keep".into(),
        }
    }
}

impl Panel {
    /// The style, width, color and edge rows.
    pub(super) fn border_rows(&self, cx: &Context<Self>) -> [(SharedString, AnyElement); 4] {
        let label = |key: &str| Book::text(cx, key, &[]);
        let buttons = PRESETS.iter().map(|(marks, key)| self.edge_button(marks, key, cx));
        [
            (label("table.format.line_style"), self.line_style(cx)),
            (label("table.format.line_width"), self.line_width(cx)),
            (label("table.format.line_color"), self.line_color(cx)),
            (label("table.format.borders"), row(buttons)),
        ]
    }

    /// A button drawing the line on `marks`.
    fn edge_button(&self, marks: &'static str, key: &str, cx: &Context<Self>) -> AnyElement {
        let panel = cx.entity().downgrade();
        Button::new(SharedString::from(format!("edges-{marks}")), Book::text(cx, key, &[]))
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                panel
                    .update(cx, |panel, cx| {
                        // Clearing leaves nothing to restyle.
                        panel.drawn = (!marks.contains('x'))
                            .then(|| panel.read.clone().map(|spot| (spot, marks)))
                            .flatten();
                        panel.draw(marks, cx);
                    })
                    .ok();
            })
            .into_any_element()
    }

    /// Draws the current line on the edges `marks` names.
    fn draw(&self, marks: &str, cx: &mut Context<Self>) {
        let values: Vec<String> = marks.chars().map(|mark| self.line.edge(mark)).collect();
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        run(&self.book, "cells.border", &values, cx);
    }

    /// A new line restyles what the buttons last drew.
    fn restyle(&mut self, cx: &mut Context<Self>) {
        if let Some((_, marks)) = self.drawn.clone() {
            self.draw(marks, cx);
        }
        cx.notify();
    }

    fn line_style(&self, cx: &Context<Self>) -> AnyElement {
        let choices: Vec<Choice> = STYLES
            .iter()
            .map(|(style, sample, key)| {
                let name = Book::text(cx, key, &[]);
                Choice::new(style.to_string(), format!("{sample}  {name}"))
            })
            .collect();
        let panel = cx.entity().downgrade();
        Select::new("table-line-style", choices)
            .selected(self.line.style.to_string())
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                if let Ok(style) = value.parse() {
                    panel
                        .update(cx, |panel, cx| {
                            panel.line.style = style;
                            panel.restyle(cx);
                        })
                        .ok();
                }
            })
            .into_any_element()
    }

    fn line_width(&self, cx: &Context<Self>) -> AnyElement {
        let choices: Vec<Choice> = WIDTHS
            .iter()
            .map(|(width, key)| Choice::new(width.to_string(), Book::text(cx, key, &[])))
            .collect();
        let panel = cx.entity().downgrade();
        Select::new("table-line-width", choices)
            .selected(self.line.width.to_string())
            .size(ControlSize::Sm)
            .on_change(move |value, _, cx| {
                if let Ok(width) = value.parse() {
                    panel
                        .update(cx, |panel, cx| {
                            panel.line.width = width;
                            panel.restyle(cx);
                        })
                        .ok();
                }
            })
            .into_any_element()
    }

    /// The border color as a swatch; it opens the palette.
    fn line_color(&self, cx: &Context<Self>) -> AnyElement {
        let current = self.line.color;
        let panel = cx.entity().downgrade();
        Popover::with_opener(
            "table-line-popover",
            move |toggle| {
                ColorSwatch::new("table-line-swatch", Hsla::from(rgb(current)))
                    .tooltip(format!("#{current:06X}"))
                    .on_click(move |window, cx| toggle(window, cx))
            },
            move |close, _, _| {
                let swatches = LINES.map(|color| (format!("#{color:06X}"), Hsla::from(rgb(color))));
                let panel = panel.clone();
                ColorPalette::new("table-line", swatches.clone())
                    .columns(5)
                    .selected(rgb(current))
                    .on_change(move |color, window, cx| {
                        if let Some(at) = swatches.iter().position(|(_, swatch)| *swatch == color) {
                            close(window, cx);
                            panel
                                .update(cx, |panel, cx| {
                                    panel.line.color = LINES[at];
                                    panel.restyle(cx);
                                })
                                .ok();
                        }
                    })
            },
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::Line;

    #[test]
    fn edges_follow_the_marks() {
        let line = Line { style: 1, width: 35, color: 0xFF0000 };
        assert_eq!(line.edge('1'), "1:35:16711680");
        assert_eq!(line.edge('x'), "none");
        assert_eq!(line.edge('0'), "keep");
    }

    #[test]
    fn another_workbook_is_not_restyled() {
        use crate::tests::{book, settle, update};
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut cx = gpui::TestAppContext::single();
        let Some((_window, book)) = book(&mut cx, dir.path()) else {
            eprintln!("SKIP: no LOK build");
            return;
        };
        for _ in 0..2 {
            update(&mut cx, &book, |book, cx| book.new_blank(cx));
        }
        settle(&cx, |app| {
            let tabs = &book.read(app).core.tabs;
            tabs.len() == 2 && tabs.iter().all(|tab| tab.view.read(app).settled())
        });
        let panel = cx.update(|app| book.read(app).panel.clone());
        let show = |cx: &mut gpui::TestAppContext, at: usize| {
            let mut tab = None;
            update(cx, &book, |book, _| {
                let at = &book.core.tabs[at];
                let id = at.id;
                tab = Some((at.view.clone(), at.doc.clone()));
                book.core.set_active(Some(id));
            });
            let (view, doc) = tab.expect("the tab");
            cx.update(|app| panel.update(app, |panel, cx| panel.show(Some((&view, &doc)), cx)));
        };
        show(&mut cx, 0);
        settle(&cx, |app| panel.read(app).read.is_some());
        cx.update(|app| {
            panel.update(app, |panel, cx| {
                panel.drawn = panel.read.clone().map(|spot| (spot, "100000"));
                panel.draw("100000", cx);
            })
        });
        assert!(cx.update(|app| panel.read(app).drawn.is_some()), "A's left edge");
        // Both books sit on A1 at the same revision.
        show(&mut cx, 1);
        settle(&cx, |app| panel.read(app).read.is_some());
        assert!(cx.update(|app| panel.read(app).drawn.is_none()), "B keeps nothing to restyle");
    }

    #[test]
    fn a_double_line_is_never_too_thin() {
        let line = Line { style: 3, width: 9, color: 0 };
        assert_eq!(line.edge('1'), "3:35:0");
        let thick = Line { style: 3, width: 88, color: 0 };
        assert_eq!(thick.edge('1'), "3:88:0");
    }
}
