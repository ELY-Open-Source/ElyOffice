//! Side panel widths, folds and the split, per app (§18.1).

use std::{io, time::Duration};

use ely_platform::{AppId, settings::Store};
use gpui::{App, AsyncApp, Global, Pixels, px};
use serde_json::{Value, json};

const KEY: &str = "panels";
const SETTLE: Duration = Duration::from_millis(500);

/// Widths the user dragged to; none keeps the default.
pub(crate) struct Panels {
    store: Option<Store>,
    left: Option<Pixels>,
    right: Option<Pixels>,
    /// Left, then right, folded away.
    hidden: (bool, bool),
    /// The first pane's share of a dragged split.
    split: Option<f32>,
    moves: u64,
    saved: u64,
}

impl Global for Panels {}

/// Reads `app`'s widths; a bad entry is reported, not guessed.
pub(super) fn install(app: AppId, cx: &mut App) {
    let store = Store::app(app);
    let read = store.as_ref().map_err(|error| io::Error::new(error.kind(), error.to_string())).and_then(|store| {
        let fields = store.read()?;
        parse(fields.get(KEY))
    });
    let ((left, right), hidden, split) = read.unwrap_or_else(|error| {
        log::error!("settings: panels unreadable: {error}");
        cx.global_mut::<super::Current>().app_error.insert(super::PANELS, error.to_string().into());
        ((None, None), (false, false), None)
    });
    cx.set_global(Panels { store: store.ok(), left, right, hidden, split, moves: 0, saved: 0 });
    bind(cx);
    // Quitting right after a drag still keeps the width.
    cx.on_app_quit(|cx| {
        flush(cx);
        async {}
    })
    .detach();
}

type Widths = (Option<Pixels>, Option<Pixels>);

/// Absent or null is default; else a positive width.
fn parse(saved: Option<&Value>) -> io::Result<(Widths, (bool, bool), Option<f32>)> {
    let invalid = |why: String| io::Error::new(io::ErrorKind::InvalidData, format!("settings: panels {why}"));
    let fields = match saved {
        None | Some(Value::Null) => return Ok(((None, None), (false, false), None)),
        Some(Value::Object(fields)) => fields,
        Some(other) => return Err(invalid(format!("cannot be {other}"))),
    };
    let width = |side: &str| match fields.get(side) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|n| n.is_finite() && n > 0.0) => {
            Ok(n.as_f64().map(|n| px(n as f32)))
        }
        Some(other) => Err(invalid(format!("{side} cannot be {other}"))),
    };
    let hidden = |key: &str| match fields.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(hidden)) => Ok(*hidden),
        Some(other) => Err(invalid(format!("{key} cannot be {other}"))),
    };
    let split = match fields.get("split") {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) if n.as_f64().is_some_and(|n| n > 0.0 && n < 1.0) => n.as_f64().map(|n| n as f32),
        Some(other) => return Err(invalid(format!("split cannot be {other}"))),
    };
    Ok(((width("left")?, width("right")?), (hidden("left_hidden")?, hidden("right_hidden")?), split))
}

/// Defaults in force, nothing kept: for tests.
#[cfg(any(test, feature = "test-support"))]
pub(super) fn install_defaults(cx: &mut App) {
    cx.set_global(Panels { store: None, left: None, right: None, hidden: (false, false), split: None, moves: 0, saved: 0 });
    bind(cx);
}

/// The fold keys reach every window of the app.
fn bind(cx: &mut App) {
    cx.on_action(|_: &crate::commands::ToggleLeftPanel, cx| show(true, !shown(true, cx), cx));
    cx.on_action(|_: &crate::commands::ToggleRightPanel, cx| show(false, !shown(false, cx), cx));
}

/// Whether the left or right panel shows.
pub fn shown(left: bool, cx: &App) -> bool {
    let (left_hidden, right_hidden) = cx.global::<Panels>().hidden;
    !if left { left_hidden } else { right_hidden }
}

/// Shows or folds a side panel; kept for the app.
pub fn show(left: bool, shown: bool, cx: &mut App) {
    let hidden = &mut cx.global_mut::<Panels>().hidden;
    let side = if left { &mut hidden.0 } else { &mut hidden.1 };
    if *side != shown {
        return;
    }
    *side = !shown;
    log::info!("settings: {} panel {}", if left { "left" } else { "right" }, if shown { "shown" } else { "folded" });
    changed(cx);
    cx.refresh_windows();
}

/// The user's width for the left or right panel.
pub(crate) fn width(left: bool, cx: &App) -> Option<Pixels> {
    let panels = cx.global::<Panels>();
    if left { panels.left } else { panels.right }
}

/// Takes a dragged width; saved once dragging rests.
pub(crate) fn set_width(left: bool, width: Pixels, cx: &mut App) {
    let panels = cx.global_mut::<Panels>();
    *(if left { &mut panels.left } else { &mut panels.right }) = Some(width);
    changed(cx);
}

/// The first pane's share of a two-pane split.
pub fn split(cx: &App) -> Option<f32> {
    cx.global::<Panels>().split
}

/// Takes a dragged share; saved once dragging rests.
pub fn set_split(share: f32, cx: &mut App) {
    cx.global_mut::<Panels>().split = Some(share);
    changed(cx);
}

/// Saves the panels once changes rest.
fn changed(cx: &mut App) {
    let panels = cx.global_mut::<Panels>();
    panels.moves += 1;
    let moves = panels.moves;
    cx.spawn(async move |cx: &mut AsyncApp| {
        cx.background_executor().timer(SETTLE).await;
        let Some((store, saved)) = cx.update(|cx| pending(cx).filter(|_| cx.global::<Panels>().moves == moves)) else {
            return;
        };
        let written = cx.background_executor().spawn(async move { store.set(KEY, saved) }).await;
        cx.update(|cx| settled(moves, written, cx));
    })
    .detach();
}

/// What to write while the last drag is unkept.
fn pending(cx: &App) -> Option<(Store, Value)> {
    let panels = cx.global::<Panels>();
    let store = panels.store.clone().filter(|_| panels.moves != panels.saved)?;
    let n = |w: Option<Pixels>| w.map(|w| f64::from(f32::from(w)));
    let (left_hidden, right_hidden) = panels.hidden;
    let split = panels.split.map(f64::from);
    Some((store, json!({ "left": n(panels.left), "right": n(panels.right), "left_hidden": left_hidden, "right_hidden": right_hidden, "split": split })))
}

/// Records a write; failures show and retry later.
fn settled(moves: u64, written: io::Result<()>, cx: &mut App) {
    match written {
        Ok(()) => {
            cx.global_mut::<Panels>().saved = moves;
            super::unsaved(super::PANELS, None, cx);
        }
        Err(error) => {
            log::error!("settings: panels not saved: {error}");
            super::unsaved(super::PANELS, Some(error.to_string().into()), cx);
        }
    }
}

/// Writes a pending width now; quitting cannot wait.
fn flush(cx: &mut App) {
    if let Some((store, saved)) = pending(cx) {
        let moves = cx.global::<Panels>().moves;
        let written = store.set(KEY, saved);
        settled(moves, written, cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;
    use serde_json::json;

    #[gpui::test]
    fn a_fold_is_kept_and_reads_back(cx: &mut gpui::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let store = ely_platform::settings::Store::at(dir.path().join("settings.json"));
        cx.update(|cx| {
            super::super::install_defaults(cx);
            cx.global_mut::<super::Panels>().store = Some(store.clone());
            super::show(true, false, cx);
            assert!(!super::shown(true, cx) && super::shown(false, cx));
        });
        cx.executor().advance_clock(super::SETTLE);
        cx.run_until_parked();
        let kept = store.read().unwrap();
        assert_eq!(super::parse(kept.get(super::KEY)).unwrap(), ((None, None), (true, false), None));
    }

    #[gpui::test]
    fn a_split_is_kept_and_reads_back(cx: &mut gpui::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let store = ely_platform::settings::Store::at(dir.path().join("settings.json"));
        cx.update(|cx| {
            super::super::install_defaults(cx);
            cx.global_mut::<super::Panels>().store = Some(store.clone());
            super::set_split(0.25, cx);
            assert_eq!(super::split(cx), Some(0.25));
        });
        cx.executor().advance_clock(super::SETTLE);
        cx.run_until_parked();
        let kept = store.read().unwrap();
        assert_eq!(super::parse(kept.get(super::KEY)).unwrap(), ((None, None), (false, false), Some(0.25)));
    }

    #[test]
    fn widths_and_folds_read_strictly() {
        assert_eq!(super::parse(None).unwrap(), ((None, None), (false, false), None));
        assert_eq!(super::parse(Some(&json!({"left": 300, "right": null}))).unwrap(), ((Some(px(300.0)), None), (false, false), None));
        let folded = json!({"left": null, "right": null, "left_hidden": true, "right_hidden": false, "split": 0.4});
        assert_eq!(super::parse(Some(&folded)).unwrap(), ((None, None), (true, false), Some(0.4)));
        let bad = [json!({"left": "broken", "right": 320}), json!({"left": -4}), json!([1]), json!({"left_hidden": 1}), json!({"split": 1}), json!({"split": "half"})];
        for bad in bad {
            assert!(super::parse(Some(&bad)).is_err(), "{bad}");
        }
    }
}
