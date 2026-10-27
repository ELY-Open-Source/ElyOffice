//! Plays a slide's animations or transition over the canvas (SHOW-09).

use std::{sync::Arc, time::Instant};

use ely_lok::{
    LokView,
    show::{Layers, Part},
};
use ely_office::prelude::*;
use gpui::{
    AnyElement, Context, Entity, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderImage, Styled as _, Window, div,
};
use image::{Frame, ImageBuffer, Rgba};

use crate::deck::Deck;

pub(crate) mod compose;
pub(crate) mod timeline;
pub(crate) mod transition;

/// About this wide; the height keeps the slide's shape.
const WIDTH: u32 = 1280;

/// What a preview plays.
pub enum Playing {
    /// The slide's layers and its effects on the clock.
    Effects {
        layers: Vec<(Part, Arc<RenderImage>)>,
        runs: Vec<timeline::Run>,
        /// The slide in 1/100 mm.
        slide: (f32, f32),
    },
    /// The previous slide, if any, giving way to this one.
    Transition {
        from: Option<Vec<(Part, Arc<RenderImage>)>>,
        to: Vec<(Part, Arc<RenderImage>)>,
        preset: String,
    },
}

/// Tab, slide, undo list, edits landed and pending.
type Stamp = (u64, String, Vec<ely_lok::history::Step>, u64, bool);

pub struct Preview {
    stamp: Stamp,
    pub playing: Playing,
    started: Instant,
    length: f64,
}

pub(crate) fn picture((width, height): (u32, u32), pixels: Vec<u8>) -> Arc<RenderImage> {
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixels)
        .expect("the worker paints the asked size");
    Arc::new(RenderImage::new([Frame::new(image)]))
}

/// The layers as pictures.
pub(crate) fn pictures(layers: Layers) -> Vec<(Part, Arc<RenderImage>)> {
    let size = layers.size;
    (layers.list.into_iter())
        .map(|layer| (layer.part, picture(size, layer.pixels)))
        .collect()
}

/// The preview's pixel size for a slide of `size`.
fn pixels(size: (i64, i64)) -> (u32, u32) {
    let height = (WIDTH as f64 * size.1 as f64 / size.0.max(1) as f64).round();
    (WIDTH, height.max(1.0) as u32)
}

impl Deck {
    /// The shown slide now; previews play only while it holds.
    fn stamp(&self, cx: &gpui::App) -> Option<Stamp> {
        let tab = self.active_tab()?;
        let slide = tab
            .extra
            .slides
            .list
            .get(tab.extra.slides.current)?
            .name
            .clone();
        let history = tab.extra.history.as_ref()?.0.clone();
        let view = tab.view.read(cx);
        Some((tab.id, slide, history, view.edited(), view.edit_pending()))
    }

    /// A new preview request; older ones no longer play.
    fn ask_preview(&mut self, cx: &gpui::App) -> Option<(u64, Stamp)> {
        self.preview_asked += 1;
        Some((self.preview_asked, self.stamp(cx)?))
    }

    /// Whether request `asked` from `stamp` may still play.
    fn may_play(&self, asked: u64, stamp: &Stamp, cx: &gpui::App) -> bool {
        self.preview_asked == asked && self.stamp(cx).as_ref() == Some(stamp)
    }

    /// Plays the shown slide's effects from the kernel's layers.
    pub fn preview_effects(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let (id, doc, part) = (tab.id, tab.doc.clone(), tab.extra.slides.current);
        let Some((_, effects)) = tab.extra.motion.clone() else {
            return;
        };
        let (asked, slide) = (pixels(tab.extra.size), crate::design::mm100(tab.extra.size));
        let Some((request, stamp)) = self.ask_preview(cx) else {
            return;
        };
        log::info!("show: previews the effects of slide {part}");
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            move |worker, doc| doc.slide_layers(worker, part, asked),
            move |deck: &mut Self, read: std::io::Result<Layers>, cx| match read {
                Ok(layers) if deck.may_play(request, &stamp, cx) => {
                    let layers = pictures(layers);
                    let runs = timeline::schedule(&effects);
                    let length = timeline::length(&runs);
                    let slide = (slide.0 as f32, slide.1 as f32);
                    deck.play(
                        stamp,
                        Playing::Effects {
                            layers,
                            runs,
                            slide,
                        },
                        length,
                        cx,
                    );
                }
                Ok(_) => log::info!("show: a preview for tab {id} is no longer due"),
                Err(error) => deck.engine_failed(&format!("the preview: {error}"), cx),
            },
            cx,
        );
    }

    /// Plays the shown slide's transition from the one before.
    pub fn preview_transition(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.active_tab() else {
            return;
        };
        let Some((transition, _)) = tab.extra.motion.clone() else {
            return;
        };
        let (id, doc, part) = (tab.id, tab.doc.clone(), tab.extra.slides.current);
        let asked = pixels(tab.extra.size);
        let Some((request, stamp)) = self.ask_preview(cx) else {
            return;
        };
        log::info!("show: previews the transition into slide {part}");
        ely_lok::read_off(
            &self.core.worker,
            &doc,
            move |worker, doc| {
                let from = (part > 0)
                    .then(|| doc.slide_layers(worker, part - 1, asked))
                    .transpose()?;
                Ok((from, doc.slide_layers(worker, part, asked)?))
            },
            move |deck: &mut Self, read: std::io::Result<(Option<Layers>, Layers)>, cx| match read {
                Ok((from, to)) if deck.may_play(request, &stamp, cx) => {
                    let playing = Playing::Transition {
                        from: from.map(pictures),
                        to: pictures(to),
                        preset: transition.preset.clone(),
                    };
                    deck.play(stamp, playing, transition.duration, cx);
                }
                Ok(_) => log::info!("show: a preview for tab {id} is no longer due"),
                Err(error) => deck.engine_failed(&format!("the preview: {error}"), cx),
            },
            cx,
        );
    }

    fn play(&mut self, stamp: Stamp, playing: Playing, length: f64, cx: &mut Context<Self>) {
        let layers = match &playing {
            Playing::Effects { layers, .. } => layers.len(),
            Playing::Transition { .. } => 2,
        };
        log::info!("show: plays {layers} layers for {length:.2} s");
        self.end_preview();
        self.preview = Some(Preview {
            stamp,
            playing,
            started: Instant::now(),
            length,
        });
        cx.notify();
    }

    /// Ends the preview; its layers wait to be freed.
    fn end_preview(&mut self) -> bool {
        let Some(preview) = self.preview.take() else {
            return false;
        };
        let pictures: Vec<&Arc<RenderImage>> = match &preview.playing {
            Playing::Effects { layers, .. } => layers.iter().map(|(_, image)| image).collect(),
            Playing::Transition { from, to, .. } => from.iter().flatten().chain(to).map(|(_, image)| image).collect(),
        };
        for picture in pictures {
            self.retired.push(picture.clone());
        }
        true
    }

    /// Drops a preview whose slide or document moved on.
    pub(crate) fn drop_stale_preview(&mut self, cx: &gpui::App) {
        if (self.preview.as_ref()).is_some_and(|p| self.stamp(cx).as_ref() != Some(&p.stamp)) {
            log::info!("show: the slide changed under the preview");
            self.end_preview();
        }
    }

    /// Stops a preview at once.
    pub fn stop_preview(&mut self, cx: &mut Context<Self>) {
        // Reads still out are not wanted either.
        self.preview_asked += 1;
        if self.end_preview() {
            log::info!("show: the preview stops");
            cx.notify();
        }
    }

    /// The playing preview over the slide; a press stops it.
    pub fn preview_layer(
        &mut self,
        view: &Entity<LokView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let size = self.active_tab()?.extra.size;
        self.drop_stale_preview(cx);
        let preview = self.preview.as_ref()?;
        let t = preview.started.elapsed().as_secs_f64();
        // The last frame stays a moment, then the canvas returns.
        if t > preview.length + 0.5 {
            log::info!("show: the preview ends");
            self.end_preview();
            return None;
        }
        window.request_animation_frame();
        let lok = view.read(cx);
        let (top_left, bottom_right) =
            (lok.place((0, 0)), lok.place((size.0 as i32, size.1 as i32)));
        let frame = (bottom_right.x - top_left.x, bottom_right.y - top_left.y);
        let this = cx.entity();
        // A show starts from black; effects play on the page.
        let behind = match &preview.playing {
            Playing::Transition { .. } => gpui::black(),
            Playing::Effects { .. } => ely_lok::paper(),
        };
        let content = match &preview.playing {
            Playing::Effects {
                layers,
                runs,
                slide,
            } => compose::effects(layers, runs, *slide, t, frame),
            Playing::Transition { from, to, preset } => {
                let p = if preview.length <= 0.0 {
                    1.0
                } else {
                    (t / preview.length) as f32
                };
                let paper = ely_lok::paper();
                // Exits are not read; the old slide shows everything.
                let from = (from.as_deref())
                    .map(|from| compose::slide(from, |_| true, frame, paper).into_any_element());
                compose::transition(
                    (from, to),
                    &transition::frame(preset, p),
                    frame,
                    paper,
                )
            }
        };
        Some(
            div()
                .id("show-preview")
                .absolute()
                .left(top_left.x)
                .top(top_left.y)
                .w(frame.0)
                .h(frame.1)
                .overflow_hidden()
                .bg(behind)
                .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| {
                    this.update(cx, |deck, cx| deck.stop_preview(cx))
                })
                .child(content)
                .into_any_element(),
        )
    }
}
