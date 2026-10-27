//! Headers, sizing drags and the fill handle (TABLE-02).

use ely_gpui_component::{primitives::FocusRing as _, theme::ActiveTheme as _};
use ely_lok::{
    LokView, Rect,
    sheet::{Axis, address, column_name},
};
use ely_office::prelude::*;
use gpui::{
    AnyElement, App, ClickEvent, Context, CursorStyle, Entity, InteractiveElement as _, IntoElement,
    Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _,
    Pixels, Point, Role, StatefulInteractiveElement as _, Styled as _, Window, div, px,
    transparent_black,
};

use crate::book::Book;

/// A drag over the grid.
pub enum Drag {
    /// A column or row edge; `at` the pointer along it.
    Size { column: bool, index: u32, at: Option<Pixels> },
    /// The fill handle, to the cell under the pointer.
    Fill { to: Option<(u32, u32)> },
}

/// The width that grabs a header edge.
const EDGE: f32 = 4.0;
const HANDLE: f32 = 7.0;

/// The vcl modifier bits a header click takes.
fn modifier(modifiers: &Modifiers, cx: &App) -> String {
    let shift = if modifiers.shift { 0x1000 } else { 0 };
    let add = if modifiers.secondary(cx) { 0x2000 } else { 0 };
    (shift | add).to_string()
}

/// Twips as 1/100 mm, as the size commands take them.
fn mm100(twips: i32) -> i32 {
    (twips * 2540 + 720) / 1440
}

impl Book {
    pub fn grid(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let tab = self.active_tab().expect("the grid needs a tab");
        let view = tab.view.clone();
        let (rows, columns) = (
            tab.extra.geometry.rows.clone(),
            tab.extra.geometry.columns.clone(),
        );
        let theme = cx.theme();
        let (border, sunken) = (theme.colors.border, theme.colors.sunken);
        let column_strip = self.strip(&view, &columns, true, cx);
        let row_strip = self.strip(&view, &rows, false, cx);
        let corner = div()
            .w_12()
            .h_6()
            .border_r_1()
            .border_b_1()
            .border_color(border);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(sunken)
            .on_mouse_move(cx.listener(Self::on_drag_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_drag_end))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|book, _, _, cx| {
                if book.drag.take().is_some() {
                    cx.notify();
                }
            }))
            .child(div().flex().child(corner).child(column_strip))
            .child(
                div().flex_1().min_h_0().flex().child(row_strip).child(
                    div()
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .child(view.clone())
                        .children(self.guide(&view, cx))
                        .children(self.fill_handle(&view, cx))
                        .children(self.cell_aids(&view, cx))
                        .children(self.find_box(cx)),
                ),
            )
            .into_any_element()
    }

    /// One header strip; `column` picks the axis.
    fn strip(
        &self,
        view: &Entity<LokView>,
        axis: &Axis,
        column: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let border = theme.colors.border;
        let read = view.read(cx);
        let (width, height) = read.viewport();
        let along = |twips: i32| {
            let at = read.place((twips, twips));
            f32::from(if column { at.x } else { at.y })
        };
        let length = if column { width } else { height };
        let pick = |pair: (i32, i32)| if column { pair.0 } else { pair.1 };
        let (frozen, first) = (pick(read.frozen()), pick(read.scrolled_start()));
        let mut spans = Vec::new();
        let mut take = |index: u32| {
            let (start, end) = (along(axis.start(index)), along(axis.start(index + 1)));
            if end > start {
                spans.push((index, start, end - start));
            }
        };
        // The frozen indexes, then the scrolled ones.
        let held = if frozen > 0 {
            axis.at(frozen - 1) + 1
        } else {
            0
        };
        (0..held).for_each(&mut take);
        let mut index = axis.at(first).max(held);
        while index < axis.count() && along(axis.start(index)) < length {
            take(index);
            index += 1;
        }
        let cells: Vec<AnyElement> = spans
            .into_iter()
            .map(|(index, start, size)| self.header(index, start, size, column, cx))
            .collect();
        let strip = div()
            .id(if column {
                "table-columns"
            } else {
                "table-rows"
            })
            .relative()
            .overflow_hidden()
            .border_color(border)
            .children(cells);
        if column {
            strip.flex_1().h_6().border_b_1().into_any_element()
        } else {
            strip.w_12().h_full().border_r_1().into_any_element()
        }
    }

    /// Selects a header's row or column.
    pub(crate) fn select_header(&mut self, column: bool, index: u32, modifier: &str, cx: &mut Context<Self>) {
        let id = if column { "select.column" } else { "select.row" };
        self.run_active(id, &[&index.to_string(), modifier], cx);
    }

    /// Sizes a column or row to its content.
    fn fit(&mut self, column: bool, index: u32, cx: &mut Context<Self>) {
        let id = if column { "columns.fit" } else { "rows.fit" };
        log::info!("table: {id} {}", index + 1);
        self.run_active(id, &["0", &(index + 1).to_string()], cx);
    }

    /// Where a dragged edge will land, across the sheet.
    fn guide(&self, view: &Entity<LokView>, cx: &Context<Self>) -> Option<AnyElement> {
        let Some(Drag::Size { column, at: Some(at), .. }) = self.drag else {
            return None;
        };
        let origin = view.read(cx).origin();
        let line = div()
            .absolute()
            .debug_selector(|| "table-guide".into())
            .bg(cx.theme().colors.focus);
        let line = match column {
            true => line.top_0().bottom_0().w(px(1.)).left(at - origin.x),
            false => line.left_0().right_0().h(px(1.)).top(at - origin.y),
        };
        Some(line.into_any_element())
    }

    /// One row or column header with its sizing edge.
    fn header(
        &self,
        index: u32,
        start: f32,
        size: f32,
        column: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (border, text) = (theme.colors.border, theme.colors.fg_muted);
        let label = if column {
            column_name(index)
        } else {
            (index + 1).to_string()
        };
        let edge = div()
            .id(("table-edge", index as usize))
            .absolute()
            .cursor(if column {
                CursorStyle::ResizeLeftRight
            } else {
                CursorStyle::ResizeUpDown
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |book, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    // A double click fits the content (#15).
                    if event.click_count == 2 {
                        book.drag = None;
                        return book.fit(column, index, cx);
                    }
                    book.drag = Some(Drag::Size { column, index, at: None });
                }),
            );
        let edge = if column {
            edge.top_0().right_0().h_full().w(px(EDGE))
        } else {
            edge.left_0().bottom_0().w_full().h(px(EDGE))
        };
        let id = (
            if column { "table-column" } else { "table-row" },
            index as usize,
        );
        let name = if column {
            Book::text(cx, "table.header.column", &[("name", &label)])
        } else {
            Book::text(cx, "table.header.row", &[("name", &label)])
        };
        let button = div()
            .id(id)
            .role(if column {
                Role::ColumnHeader
            } else {
                Role::RowHeader
            })
            .aria_label(name)
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(transparent_black())
            .focus_ring(cx)
            .tab_index(0)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |book, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    book.header_menu(column, index, event.position, cx);
                }),
            )
            .on_click(cx.listener(move |book, event: &ClickEvent, _, cx| {
                book.select_header(column, index, &modifier(&event.modifiers(), cx), cx);
            }))
            .child(label);
        let cell = div()
            .absolute()
            .overflow_hidden()
            .text_xs()
            .text_color(text)
            .border_color(border)
            .child(button)
            .child(edge);
        let cell = if column {
            cell.top_0()
                .left(px(start))
                .w(px(size))
                .h_full()
                .border_r_1()
        } else {
            cell.left_0()
                .top(px(start))
                .h(px(size))
                .w_full()
                .border_b_1()
        };
        cell.into_any_element()
    }

    /// The square at the selection's corner; dragging it fills.
    fn fill_handle(&self, view: &Entity<LokView>, cx: &mut Context<Self>) -> Option<AnyElement> {
        let read = view.read(cx);
        if read.cell_text_selected() {
            return None;
        }
        let area = span(read.selection()).or(read.cell().cursor)?;
        if !read.is_visible(area) {
            return None;
        }
        let corner = read.place((area.0 + area.2, area.1 + area.3));
        let start = read.place((area.0, area.1));
        // Scrolled under the frozen part, the area shows nothing.
        if corner.x <= start.x || corner.y <= start.y {
            return None;
        }
        let theme = cx.theme();
        let (fill, edge) = (theme.colors.focus, theme.colors.bg);
        let preview = match &self.drag {
            Some(Drag::Fill {
                to: Some((column, row)),
            }) => {
                let geometry = &self.active_tab()?.extra.geometry;
                let end = (
                    geometry.columns.start(column + 1),
                    geometry.rows.start(row + 1),
                );
                Some(outline(read.place((area.0, area.1)), read.place(end), fill))
            }
            _ => None,
        };
        let handle = div()
            .id("table-fill")
            .absolute()
            .left(corner.x - px(HANDLE / 2.0))
            .top(corner.y - px(HANDLE / 2.0))
            .size(px(HANDLE))
            .bg(fill)
            .border_1()
            .border_color(edge)
            .cursor(CursorStyle::Crosshair)
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|book, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    book.drag = Some(Drag::Fill { to: None });
                    cx.notify();
                }),
            );
        let layer = div().absolute().top_0().left_0().size_full();
        Some(layer.children(preview).child(handle).into_any_element())
    }

    fn on_drag_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(Drag::Size { column, at, .. }) = &mut self.drag {
            *at = Some(if *column { event.position.x } else { event.position.y });
            return cx.notify();
        }
        if !matches!(self.drag, Some(Drag::Fill { .. })) {
            return;
        }
        let Some(tab) = self.active_tab() else {
            return;
        };
        let read = tab.view.read(cx);
        let (x, y) = read.twips_at(event.position - read.origin());
        let geometry = &tab.extra.geometry;
        let cell = (geometry.columns.at(x), geometry.rows.at(y));
        self.drag = Some(Drag::Fill { to: Some(cell) });
        cx.notify();
    }

    fn on_drag_end(&mut self, event: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        cx.notify();
        match drag {
            // A press without a move sizes nothing.
            Drag::Size { at: None, .. } => {}
            Drag::Size { column, index, .. } => {
                let Some(tab) = self.active_tab() else {
                    return;
                };
                let read = tab.view.read(cx);
                let geometry = &tab.extra.geometry;
                let axis = if column {
                    &geometry.columns
                } else {
                    &geometry.rows
                };
                let local = event.position - read.origin();
                let (x, y) = read.twips_at(local);
                let twips = (if column { x } else { y }) - axis.start(index);
                let size = mm100(twips).clamp(1, 65535).to_string();
                let id = if column {
                    "columns.width"
                } else {
                    "rows.height"
                };
                log::info!("table: {id} {} to {size}", index + 1);
                self.run_active(id, &[&size, &(index + 1).to_string()], cx);
            }
            Drag::Fill {
                to: Some((column, row)),
            } => {
                self.run_active("fill.auto", &[&address(column, row)], cx);
            }
            Drag::Fill { to: None } => {}
        }
    }
}

/// The rectangle that holds all of `rects`.
fn span(rects: &[Rect]) -> Option<Rect> {
    let first = *rects.first()?;
    let (mut x0, mut y0, mut x1, mut y1) = (first.0, first.1, first.0 + first.2, first.1 + first.3);
    for &(x, y, w, h) in &rects[1..] {
        (x0, y0) = (x0.min(x), y0.min(y));
        (x1, y1) = (x1.max(x + w), y1.max(y + h));
    }
    Some((x0, y0, x1 - x0, y1 - y0))
}

/// A frame from `from` to `to`, view-local.
fn outline(from: Point<Pixels>, to: Point<Pixels>, color: gpui::Hsla) -> AnyElement {
    div()
        .absolute()
        .left(from.x)
        .top(from.y)
        .w(to.x - from.x)
        .h(to.y - from.y)
        .border_2()
        .border_color(color)
        .into_any_element()
}
