//! Page pictures, and bookmarks if any (PDF-01, PDF-02).

use std::{collections::BTreeSet, ops::Range, sync::Arc};

use ely_gpui_component::{
    buttons::SegmentedControl,
    menus::{ContextMenu, Menu as Popup, MenuItem as PopupItem},
    primitives::{DragGhost, FocusRing as _, IconName},
    theme::{ActiveTheme as _, ControlSize},
};
use gpui::{
    Action, App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Render, RenderImage, Role,
    StatefulInteractiveElement as _, Styled as _, WeakEntity, Window, div, img, prelude::FluentBuilder as _, px,
    uniform_list,
};

use crate::organize::{CropPages, DeletePages, ExtractPages, InsertBlankPage, InsertPagesFromFile, RotatePagesLeft, RotatePagesRight};
use crate::view::PdfView;
use crate::viewer::Viewer;

/// The key context of the page list.
pub const PAGES: &str = "PdfPages";
/// A picture's width in pixels, and its row's height.
const WIDTH: f32 = 96.0;
const ROW: f32 = 152.0;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Pane {
    Pages,
    Marks,
    Notes,
    Files,
}

/// Pages dragged in the list.
#[derive(Clone)]
struct Dragged {
    view: gpui::EntityId,
    pages: BTreeSet<usize>,
}

pub struct Side {
    viewer: WeakEntity<Viewer>,
    pub(crate) pane: Pane,
    /// The view shown, watched for new pictures and edits.
    watched: Option<(gpui::EntityId, gpui::Subscription)>,
}

impl Side {
    pub fn new(viewer: &Entity<Viewer>, cx: &mut Context<Self>) -> Self {
        cx.observe(viewer, |_, _, cx| cx.notify()).detach();
        Side { viewer: viewer.downgrade(), pane: Pane::Pages, watched: None }
    }

    /// Follows `view`, dropping the one followed before.
    fn watch(&mut self, view: &Entity<PdfView>, cx: &mut Context<Self>) {
        if self.watched.as_ref().is_some_and(|(id, _)| *id == view.entity_id()) {
            return;
        }
        let watching = cx.observe(view, |_, _, cx| cx.notify());
        self.watched = Some((view.entity_id(), watching));
    }
}

/// The page commands on a right click.
fn popup(cx: &App) -> Popup {
    let item = |key: &str, action: Box<dyn Action>| {
        PopupItem::new(Viewer::text(cx, key, &[]))
            .on_click(move |window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    Popup::new()
        .item(item("pdf.pages.blank", Box::new(InsertBlankPage)))
        .item(item("pdf.pages.file", Box::new(InsertPagesFromFile)))
        .item(item("pdf.pages.delete", Box::new(DeletePages)))
        .separator()
        .item(item("pdf.pages.rotate_left", Box::new(RotatePagesLeft)))
        .item(item("pdf.pages.rotate_right", Box::new(RotatePagesRight)))
        .item(item("pdf.pages.crop", Box::new(CropPages)))
        .item(item("pdf.pages.extract", Box::new(ExtractPages)))
}

impl PdfView {
    /// Picks `page` as a click with these keys does.
    pub fn pick(&mut self, page: usize, (range, toggle): (bool, bool), cx: &mut Context<Self>) {
        match (range, self.anchor) {
            (true, Some(anchor)) => self.picked = (anchor.min(page)..=anchor.max(page)).collect(),
            _ if toggle => {
                if !self.picked.remove(&page) {
                    self.picked.insert(page);
                }
                self.anchor = Some(page);
            }
            _ => {
                self.picked = BTreeSet::from([page]);
                self.anchor = Some(page);
                self.show(page, 0.0, cx);
            }
        }
        cx.notify();
    }

    /// Asks for the first missing picture in `rows`.
    fn ask_thumb(&mut self, rows: Range<usize>, sf: f32, cx: &mut Context<Self>) {
        if self.thumbing {
            return;
        }
        let due = |page: &usize| !self.thumbs.contains_key(page) || self.thumbs_stale.contains(page);
        let Some(page) = rows.clone().find(|page| due(page) && !self.thumbs_failed.contains(page)) else {
            return;
        };
        let (w, h) = self.sizes[page];
        let scale = WIDTH * sf / w;
        let tile = [0, 0, (w * scale).ceil() as i32, (h * scale).ceil() as i32];
        let (doc, version) = (self.doc, self.version);
        self.thumbing = true;
        self.off(
            move |worker| worker.render(doc, page, (scale, 0), tile),
            move |view, painted, cx| {
                view.thumbing = false;
                if view.version != version {
                    return cx.notify();
                }
                let painted = painted.map(|drawn| {
                    if drawn.damaged {
                        view.found_damage(cx);
                    }
                    drawn.bgra
                });
                match painted.map(|bgra| crate::pages::picture((tile[2] as u32, tile[3] as u32), bgra)) {
                    Ok(Some(image)) => {
                        view.thumbs_stale.remove(&page);
                        if let Some(old) = view.thumbs.insert(page, image) {
                            view.retired.push(old);
                        }
                    }
                    Ok(None) => log::error!("pdf: a page picture came back the wrong size"),
                    Err(error) if ely_pdf_engine::is_stale(&error) => log::info!("pdf: a page picture from before an edit is dropped"),
                    Err(error) => {
                        view.thumbs_failed.insert(page);
                        view.fail("drawing a page picture", &error, cx);
                    }
                }
                cx.notify();
            },
            cx,
        );
    }
}

impl PdfView {
    /// The picture a row draws; an old one until repainted.
    pub fn picture(&self, page: usize) -> Option<Arc<RenderImage>> {
        self.thumbs.get(&page).cloned()
    }
}

/// One row: the page number and its picture.
fn row(view: &Entity<PdfView>, page: usize, cx: &App) -> gpui::AnyElement {
    let read = view.read(cx);
    let colors = &cx.theme().colors;
    let (w, h) = read.sizes[page];
    let picked = read.picked.contains(&page);
    let picture = read.picture(page);
    let count = read.sizes.len();
    let label = Viewer::text(cx, "pdf.page.label", &[("n", &(page + 1).to_string())]);
    let (clicked, pressed, started) = (view.clone(), view.clone(), view.clone());
    let aim = read.aimed.filter(|_| cx.has_active_drag());
    let dragged = Dragged {
        view: view.entity_id(),
        pages: if picked { read.picked.clone() } else { BTreeSet::from([page]) },
    };
    let ghost = label.clone();
    let height = (h * WIDTH / w).min(ROW - 24.0);
    div()
        .relative()
        .id(("pdf-thumb", page))
        .debug_selector(move || format!("pdf-thumb-{page}"))
        .w_full()
        .h(px(ROW))
        .tab_index(0)
        .focus_ring(cx)
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .pt_2()
        .cursor_pointer()
        .on_click(move |event, _, cx| {
            let keys = event.modifiers();
            clicked.update(cx, |view, cx| view.pick(page, (keys.shift, keys.secondary()), cx))
        })
        .on_mouse_down(gpui::MouseButton::Right, move |_, _, cx| {
            pressed.update(cx, |view, cx| {
                if !view.picked.contains(&page) {
                    view.pick(page, (false, false), cx);
                }
            })
        })
        .on_drag(dragged, move |_, _, window, cx| {
            started.update(cx, |view, _| view.aimed = None);
            // The move that starts a drag missed the zones.
            window.defer(cx, |window, cx| {
                let (position, modifiers) = (window.mouse_position(), window.modifiers());
                let event = gpui::MouseMoveEvent { position, pressed_button: Some(gpui::MouseButton::Left), modifiers };
                window.dispatch_event(gpui::PlatformInput::MouseMove(event), cx);
            });
            DragGhost::new(ghost.clone(), None, cx)
        })
        .child(
            div()
                .w(px(WIDTH))
                .h(px(height))
                .border_1()
                .border_color(if picked { colors.focus } else { colors.border })
                .bg(gpui::white())
                .when_some(picture, |frame, picture| frame.child(img(picture).size_full())),
        )
        .child(div().text_xs().text_color(colors.fg_subtle).child((page + 1).to_string()))
        .child(zone(view, (page, false), aim, cx))
        .child(zone(view, (page + 1, true), aim, cx))
        .role(Role::ListItem)
        .aria_label(label)
        .aria_selected(picked)
        .aria_position_in_set(page + 1)
        .aria_size_of_set(count)
        .into_any_element()
}

/// Half a row: drops land before page `to`.
fn zone(view: &Entity<PdfView>, (to, low): (usize, bool), aim: Option<(usize, bool)>, cx: &App) -> gpui::Div {
    let (aimed, dropped) = (view.clone(), view.clone());
    let zone = div().absolute().left_0().w_full().h(gpui::relative(0.5));
    let line = div().debug_selector(|| "pdf-drop".into()).absolute().left_0().w_full().h(px(2.0)).bg(cx.theme().colors.focus);
    let (zone, line) = if low { (zone.bottom_0(), line.bottom_0()) } else { (zone.top_0(), line.top_0()) };
    zone.when(aim == Some((to, low)), |zone| zone.child(line))
        .on_drag_move(move |event: &gpui::DragMoveEvent<Dragged>, _, cx| {
            if event.drag(cx).view != aimed.entity_id() {
                return;
            }
            let over = event.bounds.contains(&event.event.position);
            aimed.update(cx, |view, cx| {
                let now = match over {
                    true => Some((to, low)),
                    false if view.aimed == Some((to, low)) => None,
                    false => view.aimed,
                };
                if now != view.aimed {
                    view.aimed = now;
                    cx.notify();
                }
            })
        })
        .on_drop(move |drag: &Dragged, _, cx| {
            if drag.view != dropped.entity_id() {
                return;
            }
            let pages = drag.pages.clone();
            dropped.update(cx, |view, cx| {
                view.anchor = pages.first().copied();
                view.picked = pages;
                cx.notify();
                view.move_to(to, cx);
            })
        })
}

impl Render for Side {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(view) = self.viewer.upgrade().and_then(|viewer| viewer.read(cx).active_view()) else {
            return div().into_any_element();
        };
        self.watch(&view, cx);
        let pane = self.pane;
        let this = cx.entity();
        let value = match pane {
            Pane::Pages => "pages",
            Pane::Marks => "marks",
            Pane::Notes => "notes",
            Pane::Files => "files",
        };
        let head = SegmentedControl::new("pdf-side-pane", value)
            .size(ControlSize::Sm)
            .segment("pages", Viewer::text(cx, "pdf.side.pages", &[]), Some(IconName::FileText))
            .segment("marks", Viewer::text(cx, "pdf.side.marks", &[]), Some(IconName::Bookmark))
            .segment("notes", Viewer::text(cx, "pdf.side.notes", &[]), Some(IconName::StickyNote))
            .segment("files", Viewer::text(cx, "pdf.side.files", &[]), Some(IconName::Paperclip))
            .on_change(move |value, _, cx| {
                this.update(cx, |side, cx| {
                    side.pane = match value.as_ref() {
                        "marks" => Pane::Marks,
                        "notes" => Pane::Notes,
                        "files" => Pane::Files,
                        _ => Pane::Pages,
                    };
                    cx.notify();
                })
            });
        let body = match pane {
            Pane::Marks => crate::marks::outline(&view, cx).into_any_element(),
            Pane::Notes => crate::notelist::list(&view, cx),
            Pane::Files => crate::attach::list(&view, cx),
            Pane::Pages => {
                let sf = window.scale_factor();
                let count = view.read(cx).sizes.len();
                let rows = view.clone();
                let list = uniform_list("pdf-thumbs", count, move |range, _, cx| {
                    rows.update(cx, |view, cx| view.ask_thumb(range.clone(), sf, cx));
                    range.map(|page| row(&rows, page, cx)).collect()
                })
                .size_full();
                div()
                    .id("pdf-pages-list")
                    .size_full()
                    .role(Role::List)
                    .aria_label(Viewer::text(cx, "pdf.side.pages", &[]))
                    .key_context(PAGES)
                    .child(ContextMenu::new("pdf-thumbs-menu", popup(cx)).size_full().child(list))
                    .into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(div().p_2().child(head))
            .child(div().flex_1().min_h_0().child(body))
            .into_any_element()
    }
}
