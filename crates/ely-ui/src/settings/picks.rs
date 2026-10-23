//! Picks saved in the background; each key's latest wins.

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use ely_gpui_component::{
    buttons::SegmentedControl,
    forms::{Choice, Select},
};
use ely_platform::settings::{DOC_ZOOM, DOC_ZOOMS, DocZoom, Sides, TWO_SIDED, UI_TEXT_SIZE, UI_TEXT_SIZES, word};
use gpui::{App, AsyncApp, Global};
use serde_json::Value;

use crate::i18n::text;

/// Turns and waiting picks, app-wide: they outlive a window.
#[derive(Default)]
struct Picks {
    turns: HashMap<&'static str, Arc<AtomicU64>>,
    /// Picks whose save waits; their controls show them.
    waiting: HashMap<&'static str, Value>,
}

impl Global for Picks {}

/// Saves `key`; a later pick skips this write.
pub(super) fn pick(key: &'static str, value: Value, cx: &mut App) {
    let picks = cx.default_global::<Picks>();
    picks.waiting.insert(key, value.clone());
    let latest = Arc::clone(picks.turns.entry(key).or_default());
    let asked = latest.fetch_add(1, Ordering::SeqCst) + 1;
    let write = super::saved(key, value, Some((Arc::clone(&latest), asked)), cx);
    cx.refresh_windows();
    cx.spawn(async move |cx: &mut AsyncApp| {
        let written = write.await;
        // Saved: the file's value shows; failed: the pick stays.
        cx.update(|cx| {
            if written && latest.load(Ordering::SeqCst) == asked {
                cx.global_mut::<Picks>().waiting.remove(key);
                cx.refresh_windows();
            }
        });
    })
    .detach();
}

/// `key`'s pick still waiting to be saved.
pub(super) fn waiting(key: &'static str, cx: &App) -> Option<Value> {
    cx.try_global::<Picks>()?.waiting.get(key).cloned()
}

/// One segment per size; a waiting pick shows.
pub(super) fn text_size(stored: u32, cx: &App) -> SegmentedControl {
    let shown = waiting(UI_TEXT_SIZE, cx).and_then(|size| size.as_u64()).map_or(stored, |size| size as u32);
    let (least, most, _) = UI_TEXT_SIZES;
    (least..=most)
        .fold(SegmentedControl::new("settings-ui-size", shown.to_string()), |control, size| {
            control.segment(size.to_string(), format!("{size} px"), None)
        })
        .on_change(|picked, _, cx| pick(UI_TEXT_SIZE, Value::from(picked.parse::<u32>().expect("a size segment")), cx))
}

/// Automatic or a percent, for documents opened next.
pub(super) fn doc_zoom(stored: DocZoom, cx: &App) -> Select {
    let shown = match waiting(DOC_ZOOM, cx).unwrap_or_else(|| stored.value()) {
        Value::String(word) => word,
        percent => percent.to_string(),
    };
    let choices = std::iter::once(Choice::new("auto", text(cx, "settings.zoom.auto", &[])))
        .chain(DOC_ZOOMS.map(|percent| Choice::new(percent.to_string(), format!("{percent}%"))));
    Select::new("settings-doc-zoom", choices).selected(shown).on_change(|picked, _, cx| {
        let zoom = match picked.as_ref() {
            "auto" => DocZoom::Auto,
            percent => DocZoom::Percent(percent.parse().expect("a zoom choice")),
        };
        pick(DOC_ZOOM, zoom.value(), cx)
    })
}

/// The sides the print dialog starts on.
pub(super) fn two_sided(stored: Sides, cx: &App) -> Select {
    let shown = match waiting(TWO_SIDED, cx) {
        Some(Value::String(stored)) => stored,
        _ => word(&Sides::ALL, &stored).to_string(),
    };
    let choices = Sides::ALL.map(|(_, stored)| Choice::new(stored, text(cx, &format!("settings.two_sided.{stored}"), &[])));
    Select::new("settings-two-sided", choices)
        .selected(shown)
        .on_change(|picked, _, cx| pick(TWO_SIDED, Value::String(picked.to_string()), cx))
}
