//! What ElyShow keeps per deck.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use ely_lok::{
    Sheets,
    history::Step,
    show::{Design, Objects},
};
use gpui::RenderImage;

/// Parts, edits, caret and selected frame.
pub type Seen = (u64, u64, bool, Option<ely_lok::Rect>);

/// What ElyShow keeps per deck.
#[derive(Default)]
pub struct Extra {
    pub slides: Sheets,
    /// The document as the last read saw it.
    pub read_at: Option<ely_lok::Doc>,
    /// What the view showed at the last read.
    pub read: Option<Seen>,
    pub reading: bool,
    pub design: Option<Design>,
    pub objects: Option<Objects>,
    pub notes: Option<ely_lok::show::Notes>,
    pub remarks: Vec<ely_lok::show::Remark>,
    /// The shown slide's transition and effects; none on a master.
    pub motion: Option<(ely_lok::show::Transition, Vec<ely_lok::show::Effect>)>,
    pub media: Vec<ely_lok::show::Media>,
    /// Counts completed reads.
    pub reads: u64,
    /// The slide size in twips.
    pub size: (i64, i64),
    /// Undo and redo lists at the last read.
    pub history: Option<(Vec<Step>, Vec<Step>)>,
    /// A size sent (1/100 mm) and its input, until read.
    pub size_asked: Option<((i64, i64), u64)>,
    /// A transition sent for a slide, until read back.
    pub transition_asked: Option<(String, ely_lok::show::Transition, u64)>,
    /// Pictures by slide name; stale ones show until repainted.
    pub thumbs: HashMap<String, Arc<RenderImage>>,
    pub retired: ely_gpui_component::primitives::Retired,
    pub stale: HashSet<String>,
    /// Pictures that failed; retried after the next read.
    pub failed: HashSet<String>,
    pub painting: bool,
    /// The parts are masters.
    pub masters: bool,
    /// The pictures were painted with Ely's scheme.
    pub lit: bool,
}

impl Extra {
    /// The size to show, in 1/100 mm: asked, else read.
    pub fn shown_size(&self) -> (i64, i64) {
        self.size_asked
            .map_or_else(|| crate::design::mm100(self.size), |(size, _)| size)
    }

    /// The size input that landed before this read began.
    pub fn size_landed(&self, view: &ely_lok::LokView) -> Option<u64> {
        let (_, seq) = self.size_asked?;
        view.reached(seq).then_some(seq)
    }

    /// Forgets the asked size once read back, taken or refused.
    pub fn settle_size(&mut self, landed: Option<u64>) {
        let read = crate::design::mm100(self.size);
        if self.size_asked.is_some_and(|((w, h), seq)| {
            landed == Some(seq) || (w - read.0).abs() <= 50 && (h - read.1).abs() <= 50
        }) {
            self.size_asked = None;
        }
    }
}
