//! Tables, charts, notes and comments from the deck (C10.3).

use ely_gpui_component::forms::TextInput;
use ely_lok::show::Objects;
use ely_office::prelude::*;
use gpui::{AppContext as _, Entity, TestAppContext, Window};

use super::objects::{act, blank, objects};
use super::{settle, undo_steps, update};
use crate::deck::Deck;
use crate::dialog::Dialog;

/// Runs `f` on the deck inside its window.
pub(super) fn in_window<R>(
    cx: &mut TestAppContext,
    deck: &Entity<Deck>,
    f: impl FnOnce(&mut Deck, &mut Window, &mut gpui::Context<Deck>) -> R,
) -> R {
    let handle = cx
        .update(|app| deck.read(app).core.window)
        .expect("a window");
    cx.update_window(handle, |_, window, app| {
        deck.update(app, |deck, cx| f(deck, window, cx))
    })
    .expect("the window lives")
}

pub(super) fn type_in(cx: &mut TestAppContext, field: &Entity<TextInput>, text: &str) {
    let text = text.to_string();
    cx.update(|app| field.update(app, |field, cx| field.set_text(text, cx)));
}

fn has(cx: &TestAppContext, deck: &Entity<Deck>, done: impl Fn(&Objects) -> bool) {
    settle(cx, |app| objects(app, deck).as_ref().is_some_and(&done));
}

#[test]
fn the_chart_dialog_inserts_edits_and_refuses_words() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let chart_open = |app: &gpui::App| matches!(deck.read(app).dialog, Some(Dialog::Chart(_)));
    update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
    settle(&cx, chart_open);
    update(&mut cx, &deck, |deck, cx| deck.chart_asked(cx));
    has(&cx, &deck, |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "chart")
    });
    // Again, with the chart marked: it edits.
    update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
    settle(&cx, chart_open);
    let cell = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Chart(editing)) => {
            assert!(editing.changing, "the marked chart opens for editing");
            editing.table.as_ref().expect("a table").series[0].1[0].clone()
        }
        _ => unreachable!("a chart dialog"),
    });
    type_in(&mut cx, &cell, "lots");
    update(&mut cx, &deck, |deck, cx| deck.chart_asked(cx));
    let refused = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Chart(editing)) => editing.error.clone(),
        _ => None,
    });
    assert!(refused.is_some(), "a word in a cell keeps the dialog open");
    let steps = undo_steps(&mut cx, &deck);
    type_in(&mut cx, &cell, "9");
    update(&mut cx, &deck, |deck, cx| deck.chart_asked(cx));
    settle(&cx, |app| deck.read(app).dialog.is_none());
    assert_eq!(undo_steps(&mut cx, &deck), steps + 1, "one step");
    let value = cx.update(|app| {
        let deck = deck.read(app);
        let tab = deck.active_tab().expect("a tab");
        let mut worker = deck.core.worker.lock().expect("the worker lock");
        let read = worker
            .values(&tab.doc, ely_lok::sheet::CHART)
            .expect("reads");
        ely_lok::show::slide_chart(&read)
            .expect("parses")
            .expect("a chart")
            .data
            .expect("a table")
            .series[0]
            .1[0]
    });
    assert_eq!(value, Some(9.0));
}

#[test]
fn the_table_dialog_inserts_a_table() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.ask_table(cx));
    update(&mut cx, &deck, |deck, cx| deck.table_asked(cx));
    has(&cx, &deck, |o| {
        o.objects.iter().any(|object| object.kind == "table")
    });
}

#[test]
fn an_open_dialog_is_not_pushed_aside() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.ask_table(cx));
    update(&mut cx, &deck, |deck, cx| deck.ask_new(cx));
    // The chart dialog opens after a background read.
    update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
    cx.run_until_parked();
    let kept = cx.update(|app| matches!(deck.read(app).dialog, Some(crate::dialog::Dialog::Table { .. })));
    assert!(kept, "the table dialog stays");
}

#[test]
fn a_comment_lands_on_the_shown_slide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    in_window(&mut cx, &deck, |deck, window, cx| {
        deck.ask_remark(None, window, cx)
    });
    let (tab, edit) = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Remark { tab, edit, .. }) => (*tab, edit.clone()),
        _ => unreachable!("a comment dialog"),
    });
    update(&mut cx, &deck, |deck, cx| {
        deck.send_remark((tab, edit), " Too much text ", cx)
    });
    let remarks = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .map(|tab| tab.extra.remarks.clone())
            .unwrap_or_default()
    };
    settle(&cx, |app| remarks(app).len() == 1);
    let remark = cx.update(|app| remarks(app).remove(0));
    assert_eq!((remark.part, remark.text.as_str()), (0, "Too much text"));
}

#[test]
fn notes_typed_before_a_slide_change_stay_on_their_slide() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let notes = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .and_then(|tab| tab.extra.notes.clone())
    };
    update(&mut cx, &deck, |deck, cx| {
        let tab = deck.core.active;
        deck.run_in(tab, "slide.insert", &[], cx)
    });
    let shown =
        |part: usize| move |app: &gpui::App| notes(app).is_some_and(|n| n.part == Some(part));
    settle(&cx, shown(1));
    let view = |deck: &mut Deck| deck.active_tab().expect("a tab").view.clone();
    update(&mut cx, &deck, |deck, cx| {
        view(deck).update(cx, |view, _| view.show_part(0))
    });
    settle(&cx, shown(0));
    let field = in_window(&mut cx, &deck, |deck, window, cx| {
        deck.notes_input(window, cx)
    });
    type_in(&mut cx, &field.expect("the field"), "Only for the first");
    // The switch is queued before the blur's commit.
    update(&mut cx, &deck, |deck, cx| {
        view(deck).update(cx, |view, _| view.show_part(1));
        deck.commit_notes(cx);
    });
    settle(&cx, |app| {
        notes(app).is_some_and(|n| n.part == Some(1) && n.text.is_empty())
    });
    update(&mut cx, &deck, |deck, cx| {
        view(deck).update(cx, |view, _| view.show_part(0))
    });
    settle(&cx, |app| {
        notes(app).is_some_and(|n| n.part == Some(0) && n.text == "Only for the first")
    });
}

#[test]
fn a_chart_read_that_lands_late_opens_nothing() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
    settle(&cx, |app| {
        matches!(deck.read(app).dialog, Some(Dialog::Chart(_)))
    });
    update(&mut cx, &deck, |deck, cx| deck.chart_asked(cx));
    has(&cx, &deck, |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "chart")
    });
    let first = cx.update(|app| deck.read(app).core.active).expect("a tab");
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| deck.read(app).core.tabs.len() == 2);
    let second = cx.update(|app| deck.read(app).core.active).expect("a tab");
    update(&mut cx, &deck, |deck, _| deck.core.active = Some(first));
    // Another tab comes forward before the read comes back.
    update(&mut cx, &deck, |deck, cx| {
        deck.ask_chart(cx);
        deck.core.active = Some(second);
    });
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(20));
        cx.run_until_parked();
    }
    assert!(
        cx.update(|app| deck.read(app).dialog.is_none()),
        "no dialog for a hidden tab"
    );
}

#[test]
fn a_chart_read_never_shows_another_chart_on_the_same_frame() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let charts = |o: &Objects| -> Vec<usize> {
        let at = o.objects.iter().enumerate();
        at.filter(|(_, o)| o.kind == "chart")
            .map(|(at, _)| at)
            .collect()
    };
    let title = |app: &gpui::App| match &deck.read(app).dialog {
        Some(Dialog::Chart(editing)) => Some(editing.fields.title.read(app).text().to_string()),
        _ => None,
    };
    for (n, name) in [(1, "First"), (2, "Second")] {
        act(&mut cx, &deck, ("object.deselect", &["false"]), |o| {
            o.selection.is_none()
        });
        update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
        settle(&cx, |app| title(app).is_some());
        let field = cx.update(|app| match &deck.read(app).dialog {
            Some(Dialog::Chart(editing)) => editing.fields.title.clone(),
            _ => unreachable!("a chart dialog"),
        });
        type_in(&mut cx, &field, name);
        update(&mut cx, &deck, |deck, cx| deck.chart_asked(cx));
        has(&cx, &deck, |o| charts(o).len() == n);
    }
    let both = cx.update(|app| charts(&objects(app, &deck).expect("objects")));
    let first = both[0].to_string();
    act(&mut cx, &deck, ("object.select", &[&first, "false"]), |o| {
        o.objects[both[0]].selected
    });
    // The kernel picks the other chart before the read runs.
    let (worker, doc) = cx.update(|app| {
        let deck = deck.read(app);
        (
            deck.core.worker.clone(),
            deck.active_tab().expect("a tab").doc.clone(),
        )
    });
    let mut held = worker.lock().expect("the worker lock");
    update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
    let second = both[1].to_string();
    let doc = held.now(&doc).expect("the state");
    held.run(&doc, "object.select", &[&second, "false"])
        .expect("selects");
    drop(held);
    for _ in 0..100 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(20));
        cx.run_until_parked();
    }
    let shown = cx.update(|app| title(app));
    assert!(
        shown.is_none() || shown.as_deref() == Some("First"),
        "{shown:?}"
    );
}

#[test]
fn a_scatter_with_words_for_x_keeps_its_draft_and_names_the_cell() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.ask_chart(cx));
    let open = |app: &gpui::App| matches!(deck.read(app).dialog, Some(Dialog::Chart(_)));
    settle(&cx, open);
    update(&mut cx, &deck, |deck, cx| {
        if let Some(Dialog::Chart(editing)) = &mut deck.dialog {
            editing.look.kind = "scatter".into();
        }
        deck.chart_asked(cx);
    });
    let error = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Chart(editing)) => editing.error.clone(),
        _ => None,
    });
    let first = cx.update(|app| Deck::text(app, "show.chart.category", &[("n", "1")]));
    assert!(
        error.is_some_and(|e| e.contains(first.as_ref())),
        "the dialog stays and names it"
    );
    let fields = cx.update(|app| match &deck.read(app).dialog {
        Some(Dialog::Chart(editing)) => editing.table.as_ref().expect("a table").categories.clone(),
        _ => unreachable!("a chart dialog"),
    });
    for (at, field) in fields.iter().enumerate() {
        type_in(&mut cx, field, &(10 * (at + 1)).to_string());
    }
    update(&mut cx, &deck, |deck, cx| deck.chart_asked(cx));
    has(&cx, &deck, |o| {
        o.selection.as_ref().is_some_and(|s| s.kind == "chart")
    });
}

#[test]
fn a_comment_picked_on_another_slide_shows_its_slide_again() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let tab = cx.update(|app| deck.read(app).core.active).expect("a tab");
    update(&mut cx, &deck, |deck, cx| {
        deck.send_remark((tab, None), "Here", cx)
    });
    let remark = |app: &gpui::App| {
        let tab = deck.read(app).active_tab()?;
        tab.extra.remarks.first().map(|remark| remark.id.clone())
    };
    settle(&cx, |app| remark(app).is_some());
    let key: gpui::SharedString = cx.update(|app| remark(app)).expect("a comment").into();
    let current = |app: &gpui::App| {
        deck.read(app)
            .active_tab()
            .map(|tab| tab.extra.slides.current)
    };
    update(&mut cx, &deck, |deck, cx| deck.pick_remark(&key, cx));
    assert_eq!(
        cx.update(|app| deck.read(app).shown_remark()),
        Some(key.clone())
    );
    update(&mut cx, &deck, |deck, cx| {
        deck.run_in(Some(tab), "slide.insert", &[], cx)
    });
    settle(&cx, |app| current(app) == Some(1));
    assert_eq!(
        cx.update(|app| deck.read(app).shown_remark()),
        None,
        "pickable again"
    );
    update(&mut cx, &deck, |deck, cx| deck.pick_remark(&key, cx));
    settle(&cx, |app| current(app) == Some(0));
    assert_eq!(cx.update(|app| deck.read(app).shown_remark()), Some(key));
}

#[test]
fn a_zoom_stays_through_later_reports() {
    use ely_gpui_component::shell::ZoomStep;
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let view = |app: &gpui::App| deck.read(app).active_tab().expect("a tab").view.clone();
    settle(&cx, |app| view(app).read(app).fitting());
    update(&mut cx, &deck, |deck, cx| deck.zoom(ZoomStep::In, cx));
    let zoom = cx.update(|app| view(app).read(app).zoom());
    update(&mut cx, &deck, |deck, cx| {
        let (id, view) = deck.active_tab().map(|tab| (tab.id, tab.view.clone())).expect("a tab");
        deck.observed(id, &view, cx);
    });
    cx.update(|app| {
        let view = view(app);
        assert!(!view.read(app).fitting(), "a report does not fit the slide again");
        assert_eq!(view.read(app).zoom(), zoom, "the zoom stays");
    });
}

#[test]
fn a_check_row_does_not_jump_among_masters() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| deck.masters(true, cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some_and(|tab| tab.extra.masters));
    update(&mut cx, &deck, |deck, cx| {
        deck.dialog = Some(crate::dialog::Dialog::Check(Vec::new()));
        deck.go_to_finding(0, cx);
    });
    assert!(cx.update(|app| deck.read(app).dialog.is_some()), "the check stays open");
}

#[test]
fn a_refused_size_does_not_stay_shown() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let before = cx.update(|app| deck.read(app).active_tab().expect("a tab").extra.size);
    update(&mut cx, &deck, |deck, cx| deck.resize((-500, -500), cx));
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| tab.extra.size_asked.is_none())
    });
    let extra = cx.update(|app| {
        let tab = deck.read(app).active_tab().expect("a tab");
        (tab.extra.size, tab.extra.shown_size())
    });
    assert_eq!(extra.0, before, "the kernel kept its size");
    assert_eq!(extra.1, crate::design::mm100(before), "the fields show it");
}

#[test]
fn a_size_the_kernel_does_not_take_clears_after_its_read() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = blank(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    update(&mut cx, &deck, |deck, cx| {
        deck.resize((30_000, 19_050), cx);
        // As if the kernel refused: it keeps another size.
        let id = deck.core.active.expect("a tab");
        let tab = deck.tab_mut(id).expect("a tab");
        let (_, seq) = tab.extra.size_asked.expect("asked");
        tab.extra.size_asked = Some(((40_000, 19_050), seq));
    });
    settle(&cx, |app| {
        deck.read(app).active_tab().is_some_and(|tab| tab.extra.size_asked.is_none())
    });
    let read = cx.update(|app| crate::design::mm100(deck.read(app).active_tab().expect("a tab").extra.size));
    assert!((read.0 - 30_000).abs() <= 50, "the kernel's own size shows: {read:?}");
}

#[test]
fn a_default_zoom_takes_the_place_of_fitting() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    let Some(deck) = super::deck(&mut cx, dir.path()) else {
        eprintln!("SKIP: no LOK build");
        return;
    };
    let default = |cx: &mut TestAppContext, zoom| cx.update(|app| app.global_mut::<ely_ui::settings::Current>().shared.doc_zoom = zoom);
    default(&mut cx, ely_platform::settings::DocZoom::Percent(50));
    update(&mut cx, &deck, |deck, cx| deck.new_blank(cx));
    settle(&cx, |app| deck.read(app).active_tab().is_some());
    // A later default leaves an open deck alone.
    default(&mut cx, ely_platform::settings::DocZoom::Auto);
    settle(&cx, |app| super::counts(app, &deck) == (1, 1));
    cx.update(|app| {
        let view = deck.read(app).active_tab().expect("a tab").view.read(app);
        assert!(!view.fitting(), "a chosen zoom is not a whole slide");
        assert_eq!(view.zoom(), 0.5);
    });
}
