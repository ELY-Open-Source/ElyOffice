use super::*;
use gpui::{Keystroke, TestAppContext};

fn settle(cx: &TestAppContext, done: impl Fn(&App) -> bool) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < deadline {
        if !cx.executor().tick() {
            cx.executor()
                .advance_clock(std::time::Duration::from_millis(20));
        }
        if cx.update(|app| done(app)) {
            return true;
        }
    }
    false
}

#[test]
fn enter_completes_the_selected_function_without_leaving_the_cell() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, _) = cx.update(|app| {
        live_view_of(
            app,
            &install,
            "book.fods",
            crate::client::tests::input::FODS,
        )
    });
    let (_, window) = cx.add_window_view(|_, _| Host(view.clone()));
    window.run_until_parked();
    window.update(|window, app| {
        let focus = view.read(app).focus_handle().clone();
        focus.focus(window, app);
    });
    assert!(settle(window, |app| !view
        .read(app)
        .cell
        .address
        .is_empty()));
    let address = window.update(|_, app| view.read(app).cell.address.clone());
    view.update(window, |view, _| view.commit(None, "=lo"));
    assert!(settle(window, |app| {
        let functions = &view.read(app).cell.functions;
        functions.iter().any(|item| item.name == "LOOKUP")
            && functions.iter().any(|item| item.name == "LOG")
    }));
    // Keep two valid offers to exercise keyboard selection.
    view.update(window, |view, cx| {
        view.cell
            .functions
            .retain(|item| item.name == "LOG" || item.name == "LOOKUP");
        cx.notify();
    });
    window.simulate_keystrokes("down enter");
    assert!(
        settle(window, |app| {
            let view = view.read(app);
            view.settled() && view.cell.formula == "=LOOKUP()" && view.cell.functions.is_empty()
        }),
        "Enter inserts the selected function"
    );
    window.update(|_, app| {
        assert_eq!(
            view.read(app).cell.address,
            address,
            "completion stays in the cell"
        );
        view.update(app, |view, _| {
            view.commit(None, "1,1,1");
        });
    });
    assert!(settle(window, |app| {
        let view = view.read(app);
        view.settled() && view.cell.formula == "=LOOKUP(1,1,1)"
    }));
    window.simulate_keystrokes("enter");
    assert!(
        settle(window, |app| {
            let view = view.read(app);
            view.settled() && view.cell.address != address
        }),
        "Enter without candidates commits and moves to the next cell"
    );
}

#[test]
fn accepting_a_candidate_consumes_it_before_the_worker_answers() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, worker) = cx.update(|app| {
        live_view_of(
            app,
            &install,
            "book.fods",
            crate::client::tests::input::FODS,
        )
    });
    cx.update(|app| view.update(app, |view, _| view.commit(None, "=look")));
    assert!(settle(&cx, |app| view
        .read(app)
        .cell
        .functions
        .iter()
        .any(|item| item.name == "LOOKUP")));
    let held = worker.lock().expect("the worker lock");
    cx.update(|app| {
        view.update(app, |view, cx| {
            let old = Events {
                revision: view.doc.rev,
                cell: view.cell.clone(),
                ..Events::default()
            };
            let before = view.sent();
            assert!(view.key_down(&Keystroke::parse("enter").expect("Enter")));
            view.apply_batch(old, None, cx);
            assert!(
                view.completion.pending(),
                "an old event must not reoffer an accepted candidate"
            );
            assert_eq!(view.sent(), before + 1);
            assert!(view.key_down(&Keystroke::parse("enter").expect("Enter")));
            assert_eq!(
                view.sent(),
                before + 2,
                "the second Enter is queued normally"
            );
        })
    });
    drop(held);
    assert!(
        settle(&cx, |app| {
            let view = view.read(app);
            view.settled() && view.cell.address == "A2"
        }),
        "the second Enter commits and moves down"
    );
}

#[test]
fn enter_or_tab_after_queued_typing_follows_the_kernel_not_the_menu() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    for (key, to) in [("enter", "A2"), ("tab", "B1")] {
        let cx = TestAppContext::single();
        let (view, worker) = cx.update(|app| {
            live_view_of(
                app,
                &install,
                "book.fods",
                crate::client::tests::input::FODS,
            )
        });
        cx.update(|app| view.update(app, |view, _| view.commit(None, "=sum")));
        assert!(settle(&cx, |app| view
            .read(app)
            .cell
            .functions
            .iter()
            .any(|item| item.name == "SUM")));
        // The worker waits; the menu still shows SUM.
        let held = worker.lock().expect("the worker lock");
        cx.update(|app| {
            view.update(app, |view, _| {
                for text in ["(", "1", ")"] {
                    view.commit(None, text);
                }
                match key {
                    "enter" => assert!(view.key_down(&Keystroke::parse(key).expect("a key"))),
                    _ => view.next_field(),
                }
            })
        });
        drop(held);
        assert!(
            settle(&cx, |app| {
                let view = view.read(app);
                view.settled() && view.cell.address == to
            }),
            "{key} commits =sum(1) and moves to {to}"
        );
    }
}

struct Host(Entity<LokView>);

impl gpui::Render for Host {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        self.0.clone()
    }
}

#[test]
fn clicking_a_function_does_not_send_mouse_input_to_the_sheet() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, _) = cx.update(|app| {
        live_view_of(
            app,
            &install,
            "book.fods",
            crate::client::tests::input::FODS,
        )
    });
    let (_, window) = cx.add_window_view(|_, _| Host(view.clone()));
    window.run_until_parked();
    view.update(window, |view, _| view.commit(None, "=look"));
    assert!(settle(window, |app| {
        let view = view.read(app);
        view.settled() && view.cell.functions.len() == 1 && view.cell.functions[0].name == "LOOKUP"
    }));
    window.run_until_parked();
    let (on, address, sent) = window.update(|_, app| {
        let view = view.read(app);
        let cell = view.screen_rect(view.cell.cursor.expect("the cell cursor"));
        (
            cell.bottom_left() + point(px(12.0), px(12.0)),
            view.cell.address.clone(),
            view.sent(),
        )
    });
    window.simulate_mouse_down(on, gpui::MouseButton::Left, Default::default());
    window.update(|_, app| {
        let view = view.read(app);
        assert_eq!(view.sent(), sent, "the sheet never receives the press");
        assert!(
            !view.dragging,
            "the candidate press never starts a sheet drag"
        );
    });
    // Change candidates without repainting the already pressed row.
    view.update(window, |view, _| {
        view.cell.functions.insert(
            0,
            crate::Function {
                name: "LOG".into(),
                signature: String::new(),
                description: String::new(),
            },
        );
    });
    window.simulate_mouse_up(on, gpui::MouseButton::Left, Default::default());
    assert!(
        settle(window, |app| {
            let view = view.read(app);
            view.settled() && view.cell.formula == "=LOOKUP()"
        }),
        "the candidate receives the click"
    );
    window.update(|_, app| {
        let view = view.read(app);
        assert_eq!(view.cell.address, address, "the active cell stays put");
        assert_eq!(view.sent(), sent + 1, "only completion reaches the sheet");
    });
}
