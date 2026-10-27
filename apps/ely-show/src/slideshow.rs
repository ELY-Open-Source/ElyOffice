//! The show: full screen, presenter view, keys (SHOW-11, 12).

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use ely_lok::{
    Doc, Worker,
    show::{Part, ShowSlide},
};
use gpui::{AnyWindowHandle, Context, RenderImage, SharedString, Task, WeakEntity};

use crate::deck::Deck;
use crate::preview::timeline::{self, Run};
use steps::Place;

mod audience;
mod keys;
mod layers;
mod media;
mod presenter;
mod stage;
pub(crate) mod start;
pub(crate) mod steps;

pub use keys::{bind, bind_keys, menu};

/// A slide's layers as pictures, bottom first.
pub type Stack = Arc<Vec<(Part, Arc<RenderImage>)>>;

/// Clicks long done: a slide shown fully built.
const BUILT: f64 = -1.0e6;

/// The slide left, as it looked when left.
pub struct Leaving {
    pub stack: Stack,
    pub runs: Vec<Run>,
    /// Seconds after its entry.
    pub at: f64,
}

/// How a step reaches its place.
#[derive(Clone, Copy, PartialEq)]
enum How {
    /// The next click group on the same slide.
    Click,
    /// The next slide, with its transition.
    Enter,
    /// Straight there: jumps, Home, End.
    Jump,
    /// Back to a slide, all its effects done.
    Back,
}

pub struct Show {
    deck: WeakEntity<Deck>,
    worker: Arc<Mutex<Worker>>,
    /// The copy the show plays; edits never reach it.
    doc: Doc,
    copy: PathBuf,
    pub slides: Vec<ShowSlide>,
    /// The slide in 1/100 mm.
    pub slide: (f32, f32),
    pixels: (u32, u32),
    pub place: Place,
    /// When each click group began, seconds after entry.
    pub clicks: Vec<f64>,
    pub entered: Instant,
    /// The slide's layers are on their way; its clock waits.
    pub waiting: bool,
    pub leaving: Option<Leaving>,
    pub black: bool,
    pub laser: bool,
    /// The pointer on the slide, as shares.
    pub pointer: Option<(f32, f32)>,
    pub began: Instant,
    layers: HashMap<usize, Stack>,
    /// Stacks no longer wanted; freed once nothing draws them.
    gone: Vec<Stack>,
    /// Per window, audience then presenter: images to free.
    pub(crate) retired: [ely_gpui_component::primitives::Retired; 2],
    asking: HashSet<usize>,
    /// Digits typed toward a jump.
    pub typed: String,
    pub windows: Vec<AnyWindowHandle>,
    /// The presenter view is open.
    pub presenter: bool,
    /// What went wrong, for the presenter.
    pub notice: Option<SharedString>,
    advance: Option<Task<()>>,
    /// Lets the left slide go once the transition ends.
    release: Option<Task<()>>,
    media: Option<media::Playing>,
    ended: bool,
}

impl Show {
    pub fn new(
        deck: WeakEntity<Deck>,
        (worker, doc, copy): (Arc<Mutex<Worker>>, Doc, PathBuf),
        (slides, first): (Vec<ShowSlide>, usize),
        (slide, pixels): ((f32, f32), (u32, u32)),
        cx: &mut Context<Self>,
    ) -> Self {
        let mut show = Show {
            deck,
            worker,
            doc,
            copy,
            slides,
            slide,
            pixels,
            place: Place::Slide {
                part: first,
                clicks: 0,
            },
            clicks: vec![0.0],
            entered: Instant::now(),
            waiting: true,
            leaving: None,
            black: false,
            laser: false,
            pointer: None,
            began: Instant::now(),
            layers: HashMap::new(),
            gone: Vec::new(),
            retired: Default::default(),
            asking: HashSet::new(),
            typed: String::new(),
            windows: Vec::new(),
            presenter: false,
            notice: None,
            advance: None,
            release: None,
            media: None,
            ended: false,
        };
        show.fetch(cx);
        show.plan_advance(cx);
        show
    }

    pub fn part(&self) -> Option<usize> {
        match self.place {
            Place::Slide { part, .. } => Some(part),
            Place::End => None,
        }
    }

    /// The next slide the show will reach.
    pub fn upcoming(&self) -> Option<usize> {
        let from = self.part()?;
        (from + 1..self.slides.len()).find(|at| !self.slides[*at].hidden)
    }

    fn go(&mut self, to: Place, how: How, cx: &mut Context<Self>) {
        self.typed.clear();
        let now = match self.waiting {
            true => 0.0,
            false => self.entered.elapsed().as_secs_f64(),
        };
        match (how, to) {
            (How::Click, Place::Slide { part, .. }) => {
                let at = steps::click_at(&self.slides[part].effects, &self.clicks, now);
                self.clicks.push(at);
            }
            (_, Place::Slide { part, clicks }) => {
                let transition = &self.slides[part].transition;
                let moving = how == How::Enter && !transition.preset.is_empty();
                self.leaving = (self.part())
                    .filter(|_| moving)
                    .and_then(|from| {
                        Some(Leaving {
                            stack: self.layers.get(&from)?.clone(),
                            runs: steps::runs(&self.slides[from].effects, &self.clicks),
                            at: now,
                        })
                    });
                let lead = match self.leaving {
                    Some(_) => transition.duration,
                    None => 0.0,
                };
                self.entered = Instant::now();
                self.waiting = !self.layers.contains_key(&part);
                self.clicks = match how {
                    How::Back => vec![BUILT; clicks + 1],
                    _ => vec![lead],
                };
                self.drop_media();
                self.notice = None;
            }
            (_, Place::End) => {
                self.leaving = None;
                self.drop_media();
                self.notice = None;
            }
        }
        log::info!("show: goes to {to:?}");
        self.place = to;
        self.fetch(cx);
        self.plan_advance(cx);
        self.plan_release(cx);
        cx.notify();
    }

    /// The slide's layers came: its clock starts.
    pub(super) fn arrived(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.waiting, false) {
            log::info!("show: the slide is drawn; its clock starts");
            self.entered = Instant::now();
            self.plan_advance(cx);
            self.plan_release(cx);
        }
    }

    /// Drops the left slide's layers when the transition ends.
    fn plan_release(&mut self, cx: &mut Context<Self>) {
        self.release = None;
        let (Some(part), Some(_)) = (self.part(), &self.leaving) else {
            return;
        };
        if self.waiting {
            return;
        }
        let rest = self.slides[part].transition.duration - self.entered.elapsed().as_secs_f64();
        self.release = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs_f64(rest.max(0.0)))
                .await;
            let dropped = this.update(cx, |show, cx| {
                show.leaving = None;
                cx.notify();
            });
            if dropped.is_err() {
                log::info!("show: ended before its transition");
            }
        }));
    }

    pub fn next(&mut self, cx: &mut Context<Self>) {
        if self.place == Place::End {
            return self.close(cx);
        }
        let to = steps::next(&self.slides, self.place);
        let how = match (self.part(), to) {
            (Some(from), Place::Slide { part, .. }) if from == part => How::Click,
            _ => How::Enter,
        };
        self.go(to, how, cx);
    }

    pub fn previous(&mut self, cx: &mut Context<Self>) {
        self.go(steps::previous(&self.slides, self.place), How::Back, cx);
    }

    /// Goes to slide `number`, counted from one.
    pub fn jump(&mut self, number: usize, cx: &mut Context<Self>) {
        if (1..=self.slides.len()).contains(&number) {
            let to = Place::Slide {
                part: number - 1,
                clicks: 0,
            };
            self.go(to, How::Jump, cx);
        } else {
            log::info!("show: no slide {number}");
            self.typed.clear();
            cx.notify();
        }
    }

    pub fn toggle_black(&mut self, cx: &mut Context<Self>) {
        self.black = !self.black;
        log::info!("show: black screen {}", self.black);
        cx.notify();
    }

    pub fn toggle_laser(&mut self, cx: &mut Context<Self>) {
        self.laser = !self.laser;
        self.pointer = None;
        log::info!("show: laser pointer {}", self.laser);
        cx.notify();
    }

    pub fn point(&mut self, at: Option<(f32, f32)>, cx: &mut Context<Self>) {
        if self.laser && self.pointer != at {
            self.pointer = at;
            cx.notify();
        }
    }

    /// When the slide's effects have all run.
    fn settled_at(&self) -> f64 {
        let Some(part) = self.part() else {
            return 0.0;
        };
        let runs = steps::runs(&self.slides[part].effects, &self.clicks);
        timeline::length(&runs).max(self.clicks.iter().copied().fold(0.0, f64::max))
    }

    /// Whether frames still change.
    pub fn moving(&self) -> bool {
        !self.waiting && self.entered.elapsed().as_secs_f64() < self.settled_at()
    }

    /// Advances by itself after the slide's time.
    fn plan_advance(&mut self, cx: &mut Context<Self>) {
        self.advance = None;
        let Some(part) = self.part() else {
            return;
        };
        let transition = &self.slides[part].transition;
        if !transition.auto || self.waiting {
            return;
        }
        let wait = (self.settled_at() - self.entered.elapsed().as_secs_f64()).max(0.0)
            + transition.time;
        self.advance = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs_f64(wait))
                .await;
            if this.update(cx, |show, cx| show.next(cx)).is_err() {
                log::info!("show: ended before it advanced");
            }
        }));
    }

    /// Closes the windows and the copy.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.ended, true) {
            return;
        }
        log::info!("show: ends");
        (self.advance, self.release) = (None, None);
        self.drop_media();
        let (windows, deck, me) = (std::mem::take(&mut self.windows), self.deck.clone(), cx.entity_id());
        start::discard(self.worker.clone(), self.doc.clone(), self.copy.clone(), cx);
        // A key may come from a window being closed.
        cx.defer(move |cx| {
            for window in windows {
                if let Err(error) = window.update(cx, |_, window, _| window.remove_window()) {
                    log::error!("show: a window did not close: {error}");
                }
            }
            if let Some(deck) = deck.upgrade() {
                deck.update(cx, |deck, cx| deck.show_ended(me, cx));
            }
        });
    }
}
