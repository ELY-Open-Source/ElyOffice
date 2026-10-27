//! LokView's element tree and paint (LOK-03, §7 tail).

use ely_gpui_component::theme::ActiveTheme;
use gpui::{
    BorderStyle, Bounds, ContentMask, Context, Corners, CursorStyle, DispatchPhase, Edges,
    ElementInputHandler, InteractiveElement, IntoElement, MouseButton, MouseMoveEvent, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, Window, canvas, div, fill, outline, px, quad, size,
    transparent_black,
};

use std::sync::Arc;

use gpui::{Pixels, RenderImage, point};

use super::{CONTEXT, LokView, TILE_TWIPS, TWIPS_PER_PIXEL, handles, input};
use crate::Rect;

/// Ely's kernel color scheme, from `ely.xcd`.
pub const SCHEME: &str = "COLOR_SCHEME_ELY_LIGHT";

/// A page's colour, the scheme's `DocColor`.
pub fn paper() -> gpui::Hsla {
    gpui::white()
}

impl LokView {
    /// Content keeps the file's colours in any app theme.
    pub(super) fn paint_light(&mut self) {
        if !self.lit {
            self.lit = true;
            log::info!("lok: paints with {SCHEME}");
            self.send(input::Input::Theme(SCHEME));
        }
    }
}

impl Render for LokView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dpi = window.scale_factor();
        if dpi != self.geom.dpi {
            self.keep_scroll(|view| view.geom.dpi = dpi);
            self.retire_tiles();
        }
        self.paint_light();
        let this = cx.entity().clone();
        let bars = Self::bars(&this);
        let focus = self.focus.clone();
        let colors = &cx.theme().colors;
        let paper = super::paper();
        // Opaque overlays hide glyphs already painted in the tiles.
        let selection_color = colors.selection.alpha(colors.selection.a.min(0.3));
        let caret_color = colors.focus;
        let canvas_color = colors.sunken;
        let caret_width = cx.theme().caret_width().to_pixels(window.rem_size());
        self.handle_size = cx.theme().handle_hit().to_pixels(window.rem_size());
        let (frame_color, handle_fill) = (colors.focus, colors.bg);
        div()
            .id("lok-view")
            .role(Role::MultilineTextInput)
            .aria_value(self.a11y.text.clone())
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .cursor(self.border_pointer().unwrap_or(self.pointer))
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(canvas_color)
            .on_action(cx.listener(Self::on_copy))
            .on_action(cx.listener(Self::on_cut))
            .on_action(cx.listener(Self::on_next_field))
            .on_action(cx.listener(Self::on_previous_field))
            .on_action(cx.listener(Self::on_paste))
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up_out))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_scroll(cx.listener(Self::on_wheel))
            .on_pinch(cx.listener(Self::on_pinch))
            .child(
                canvas(
                    {
                        let this = this.clone();
                        move |bounds, window, app| {
                            this.update(app, |view, _| {
                                view.retired.release(window);
                                view.prepare(bounds)
                            })
                        }
                    },
                    move |bounds, painted, window, app| {
                        let mut drawn = false;
                        for page in &painted.pages {
                            window.paint_quad(fill(*page, paper));
                            window.with_content_mask(
                                Some(ContentMask { bounds: *page }),
                                |window| {
                                    for (rect, image, clip) in &painted.tiles {
                                        let mask = Some(ContentMask { bounds: *clip });
                                        let painted = window.with_content_mask(mask, |window| {
                                            window.paint_image(
                                                *rect,
                                                *rect,
                                                Corners::default(),
                                                image.clone(),
                                                0,
                                                false,
                                            )
                                        });
                                        match painted {
                                            Ok(()) => {
                                                drawn = true;
                                                let tile = Arc::as_ptr(image) as usize;
                                                if this.read(app).clock.awaits(tile) {
                                                    this.update(app, |view, _| view.clock.drawn(tile));
                                                }
                                            }
                                            Err(error) => log::error!("lok: tile paint failed: {error}"),
                                        }
                                    }
                                },
                            );
                        }
                        if drawn && !this.read(app).drawn {
                            this.update(app, |view, _| view.drawn = true);
                            log::info!("lok: first tile drawn");
                        }
                        window.with_content_mask(Some(ContentMask { bounds }), |window| {
                            for rect in painted.selection {
                                window.paint_quad(fill(rect, selection_color));
                            }
                            for (mark, color) in painted.marks {
                                window.paint_quad(quad(
                                    mark,
                                    Corners::default(),
                                    transparent_black(),
                                    Edges::all(px(2.0)),
                                    color,
                                    BorderStyle::Solid,
                                ));
                            }
                            if let Some(cell) = painted.cell {
                                window.paint_quad(quad(
                                    cell,
                                    Corners::default(),
                                    transparent_black(),
                                    Edges::all(px(2.0)),
                                    frame_color,
                                    BorderStyle::Solid,
                                ));
                            }
                            if let Some((frame, handles)) = painted.frame {
                                window.paint_quad(outline(frame, frame_color, BorderStyle::Solid));
                                for handle in handles {
                                    window.paint_quad(quad(
                                        handle,
                                        Corners::default(),
                                        handle_fill,
                                        Edges::all(px(1.0)),
                                        frame_color,
                                        BorderStyle::Solid,
                                    ));
                                }
                            }
                            if let Some(guide) = painted.guide {
                                window.paint_quad(fill(guide, frame_color));
                            }
                            if let Some(caret) = painted.caret.filter(|_| focus.is_focused(window))
                            {
                                let caret = Bounds {
                                    size: size(caret_width, caret.size.height),
                                    ..caret
                                };
                                window.paint_quad(fill(caret, caret_color));
                            }
                        });
                        {
                            // Always on: press and moves may share one frame.
                            let this = this.clone();
                            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, app| {
                                let left = event.pressed_button == Some(MouseButton::Left);
                                if phase == DispatchPhase::Bubble && left && this.read(app).dragging {
                                    this.update(app, |view, cx| {
                                        view.drag_to(event.position, event.modifiers, cx)
                                    });
                                }
                            });
                        }
                        if painted.guide.is_some() {
                            // A held border follows the pointer out of the view.
                            let this = this.clone();
                            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, app| {
                                if phase == DispatchPhase::Bubble {
                                    this.update(app, |view, cx| {
                                        if view.move_border(event.position) {
                                            cx.notify();
                                        }
                                    });
                                }
                            });
                        }
                        window.handle_input(&focus, ElementInputHandler::new(bounds, this), app);
                    },
                )
                .size_full(),
            )
            .children(self.hints(cx))
            .children(bars)
    }
}

/// The page mask, tiles, selection and caret to paint.
pub(super) struct Painted {
    /// Page frames on screen; tiles show only inside them.
    pub(super) pages: Vec<Bounds<Pixels>>,
    /// Each tile, its place and the pane that clips it.
    pub(super) tiles: Vec<(Bounds<Pixels>, Arc<RenderImage>, Bounds<Pixels>)>,
    pub(super) selection: Vec<Bounds<Pixels>>,
    pub(super) caret: Option<Bounds<Pixels>>,
    /// A spreadsheet's cell cursor.
    pub(super) cell: Option<Bounds<Pixels>>,
    /// The selected object's frame and handle squares.
    pub(super) frame: Option<(Bounds<Pixels>, Vec<Bounds<Pixels>>)>,
    /// The references of a formula being typed.
    pub(super) marks: Vec<(Bounds<Pixels>, gpui::Hsla)>,
    /// A held table border's guide line.
    pub(super) guide: Option<Bounds<Pixels>>,
}

impl LokView {
    /// The zoom that fits the page, with a margin.
    fn fit_zoom(&mut self) {
        const MARGIN: f32 = 0.94;
        let (w, h) = self.doc_size;
        if w <= 0 || h <= 0 || self.viewport.0 <= 0.0 || self.viewport.1 <= 0.0 {
            return;
        }
        let across = self.viewport.0 * TWIPS_PER_PIXEL / w as f32;
        let down = self.viewport.1 * TWIPS_PER_PIXEL / h as f32;
        let zoom = (across.min(down) * MARGIN).min(4.0);
        if (zoom - self.geom.zoom).abs() > 0.001 {
            self.keep_scroll(|view| view.geom.zoom = zoom);
            self.retire_tiles();
            self.failed.clear();
        }
    }

    /// All its images, for a closing tab to free.
    pub fn retire_all(&mut self) -> ely_gpui_component::primitives::Retired {
        self.retire_tiles();
        std::mem::take(&mut self.retired)
    }

    /// Every tile goes; their images are freed next paint.
    pub(super) fn retire_tiles(&mut self) {
        for (_, tile) in self.tiles.drain() {
            self.retired.push(tile.image);
        }
    }

    /// Drops tiles over a screen away from any shown one.
    fn keep_near(&mut self, (vw, vh): (i32, i32)) {
        let reach = (vw / TILE_TWIPS + 1, vh / TILE_TWIPS + 1);
        let shown = &self.on_screen;
        let before = self.tiles.len();
        let retired = &mut self.retired;
        self.tiles.retain(|key, tile| {
            let near = shown
                .iter()
                .any(|at| (key.0 - at.0).abs() <= reach.0 && (key.1 - at.1).abs() <= reach.1);
            if !near {
                retired.push(tile.image.clone());
            }
            near
        });
        if self.tiles.len() < before {
            log::debug!("lok: dropped {} far tiles", before - self.tiles.len());
        }
    }

    /// Room beside a narrow document, which sits centred.
    pub(super) fn slack(&self) -> (f32, f32) {
        // A sheet's grid grows with the view; it starts left.
        if self.doc.is_sheet() {
            return (0.0, 0.0);
        }
        let extent = self.extent();
        let slack = |view: f32, doc: i32| (view - self.geom.px_of(doc)).max(0.0) / 2.0;
        // A fitted page sits centred both ways.
        let down = if self.fit { slack(self.viewport.1, extent.1) } else { 0.0 };
        (slack(self.viewport.0, extent.0), down)
    }

    /// The page mask and visible tiles; requests the missing ones.
    pub(super) fn prepare(&mut self, bounds: Bounds<Pixels>) -> Painted {
        self.viewport = (bounds.size.width.into(), bounds.size.height.into());
        self.origin = bounds.origin;
        if self.fit {
            self.fit_zoom();
        }
        self.clamp_scroll();
        let geom = self.geom;
        let (sx, sy) = self.scroll;
        let extent = self.extent();
        self.spare = self.slack();
        let pages = if self.pages.is_empty() {
            vec![self.tile_screen_rect((0, 0, extent.0, extent.1))]
        } else {
            self.pages
                .iter()
                .map(|&page| self.screen_rect(page))
                .collect()
        };
        let (vw, vh) = (
            geom.twips_of(bounds.size.width.into()),
            geom.twips_of(bounds.size.height.into()),
        );
        let scale = (geom.tile_pixels(), TILE_TWIPS);
        if self.shown != Some(((sx, sy, vw, vh), scale)) {
            self.shown = Some(((sx, sy, vw, vh), scale));
            self.failed.clear();
            self.send(input::Input::Visible((sx, sy, vw, vh), scale));
        }
        let (ax, ay) = self.axes();
        let side = px(geom.px_of(TILE_TWIPS));
        let at = |vx: i32, vy: i32| {
            bounds.origin
                + point(
                    px(self.spare.0 + geom.px_of(vx)),
                    px(self.spare.1 + geom.px_of(vy)),
                )
        };
        let mut ready = Vec::new();
        let mut missing = Vec::new();
        self.on_screen.clear();
        for (dx0, dx1, vx) in ax.panes(vw) {
            for (dy0, dy1, vy) in ay.panes(vh) {
                let clip = Bounds::from_corners(at(vx, vy), at(vx + dx1 - dx0, vy + dy1 - dy0));
                let (kx0, ky0) = (dx0.div_euclid(TILE_TWIPS), dy0.div_euclid(TILE_TWIPS));
                let (kx1, ky1) = (dx1.div_euclid(TILE_TWIPS), dy1.div_euclid(TILE_TWIPS));
                for key in (kx0..=kx1).flat_map(|kx| (ky0..=ky1).map(move |ky| (kx, ky))) {
                    if key.0 * TILE_TWIPS >= extent.0 || key.1 * TILE_TWIPS >= extent.1 {
                        continue;
                    }
                    self.on_screen.push(key);
                    match self.tiles.get(&key) {
                        Some(tile) => {
                            let origin =
                                at(vx + key.0 * TILE_TWIPS - dx0, vy + key.1 * TILE_TWIPS - dy0);
                            let place = Bounds {
                                origin,
                                size: size(side, side),
                            };
                            ready.push((place, Arc::clone(&tile.image), clip));
                            if tile.stale {
                                missing.push(key);
                            }
                        }
                        None => missing.push(key),
                    }
                }
            }
        }
        self.keep_near((vw, vh));
        for key in missing {
            self.request_tile(key);
        }
        let rect = |rect: Rect| self.screen_rect(rect);
        let caret = self.text_caret().filter(|_| self.cursor_visible);
        let half = self.handle_size / 2.0;
        let frame = self.frame().map(|frame| {
            let squares = handles::points(frame)
                .iter()
                .map(|&(x, y)| Bounds {
                    origin: self.window_point((x, y)) - point(half, half),
                    size: size(self.handle_size, self.handle_size),
                })
                .collect();
            (rect(frame), squares)
        });
        Painted {
            pages,
            tiles: ready,
            selection: self.text_selection(),
            caret,
            cell: self.cell.cursor.map(rect),
            frame,
            marks: self.marks(),
            guide: self.guide(),
        }
    }
}

/// The cursor for a kernel CSS pointer name.
pub(super) fn pointer(css: &str) -> CursorStyle {
    match css {
        "default" | "wait" | "help" => CursorStyle::Arrow,
        "text" => CursorStyle::IBeam,
        "vertical-text" => CursorStyle::IBeamCursorForVerticalLayout,
        "crosshair" | "fill" => CursorStyle::Crosshair,
        "move" => CursorStyle::ClosedHand,
        "grab" => CursorStyle::OpenHand,
        "pointer" => CursorStyle::PointingHand,
        "n-resize" | "s-resize" => CursorStyle::ResizeUpDown,
        "e-resize" | "w-resize" => CursorStyle::ResizeLeftRight,
        "ne-resize" | "sw-resize" => CursorStyle::ResizeUpRightDownLeft,
        "nw-resize" | "se-resize" => CursorStyle::ResizeUpLeftDownRight,
        "col-resize" => CursorStyle::ResizeColumn,
        "row-resize" => CursorStyle::ResizeRow,
        "copy" => CursorStyle::DragCopy,
        "alias" => CursorStyle::DragLink,
        "not-allowed" => CursorStyle::OperationNotAllowed,
        _ => {
            log::warn!("lok: unknown pointer {css}");
            CursorStyle::Arrow
        }
    }
}
