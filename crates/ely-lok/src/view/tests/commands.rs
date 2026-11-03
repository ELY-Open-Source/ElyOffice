use super::*;
use gpui::TestAppContext;

/// Steps the scheduler until `done` or a minute passes.
fn settle(cx: &TestAppContext, done: impl Fn(&App) -> bool) -> bool {
    let deadline = std::time::Instant::now() + crate::client::tests::HANG;
    while std::time::Instant::now() < deadline {
        for _ in 0..20 {
            cx.executor().tick();
        }
        if cx.update(|app| done(app)) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

#[test]
fn run_reaches_the_kernel_and_state_comes_back() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    cx.update(|app| {
        view.update(app, |view, _| {
            assert!(
                view.run("format.dialog", &[]).is_err(),
                "unknown ids fail fast"
            );
            assert!(
                view.run("format.size", &[]).is_err(),
                "missing values fail fast"
            );
            view.commit(None, "a");
            view.run("edit.select_all", &[]).expect("select all queues");
            view.run("format.bold", &[]).expect("bold queues");
            view.run("format.size", &["20"]).expect("size queues");
        })
    });
    let landed = settle(&cx, |app| {
        let view = view.read(app);
        view.state(".uno:Bold") == Some("true")
            && view.state(".uno:FontHeight") == Some("20")
            && view.a11y.text == "aHello 世界"
    });
    assert!(landed, "the kernel state reaches the view");
    cx.update(|app| {
        let view = view.read(app);
        assert_eq!(
            view.a11y.text, "aHello 世界",
            "the a11y text follows the paragraph"
        );
    });
}

#[test]
fn a_kernel_dialog_reaches_apps_as_closed() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let body = crate::client::tests::input::FODS;
    let (view, _) = cx.update(|app| live_view_of(app, &install, "in.fods", body));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, closed: &crate::Closed, _| {
            errors.borrow_mut().push((closed.title.clone(), closed.body.clone()))
        })
        .detach();
        view.update(app, |view, _| {
            view.key_down(&gpui::Keystroke::parse("backspace").expect("backspace"));
        });
    });
    let reported = settle(&cx, |_| !errors.borrow().is_empty());
    assert!(reported, "the blocked dialog reaches subscribers");
    let (title, body) = errors.borrow()[0].clone();
    assert_eq!((title.as_str(), body.as_str()), ("Delete Contents", ""), "a dialog, not a message box");
}

#[test]
fn a_dead_worker_reaches_apps_while_idle() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, shared) = cx.update(|app| live_view(app, &install));
    let errors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    cx.update(|app| {
        let errors = errors.clone();
        app.subscribe(&view, move |_, error: &LokError, _| {
            errors.borrow_mut().push(error.0.clone())
        })
        .detach();
    });
    shared.lock().expect("the worker lock").child.kill().expect("the worker ends");
    let reported = settle(&cx, |_| !errors.borrow().is_empty());
    assert!(reported, "the idle watch reports the dead engine");
    assert!(
        errors.borrow()[0].contains("engine stopped"),
        "{:?}",
        errors.borrow()
    );
}

#[test]
fn a_handle_drag_resizes_the_selected_image() {
    let Some(install) = crate::client::tests::install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let cx = TestAppContext::single();
    let (view, _) = cx.update(|app| live_view(app, &install));
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(300, 200, image::Rgba([0, 0, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .expect("the png encodes");
    let path = dir.path().join("blue.png");
    std::fs::write(&path, png).expect("the png writes");
    let url = crate::file_url(&path);
    cx.update(|app| view.update(app, |view, _| view.run("image.insert", &[&url])))
        .expect("the insert queues");
    assert!(
        settle(&cx, |app| view.read(app).graphic.is_some()),
        "the image is selected"
    );
    let frame = cx.update(|app| view.read(app).graphic).expect("selected");
    let corner = super::super::handles::points(frame)[7];
    let to = (corner.0 - frame.2 / 3, corner.1 - frame.3 / 3);
    cx.update(|app| {
        view.update(app, |view, _| {
            assert!(
                view.grab(view.window_point(corner)),
                "the corner handle grabs"
            );
            view.grip = view
                .grip
                .map(|grip| super::super::handles::Grip { to, ..grip });
            assert_eq!(
                view.frame(),
                Some(super::super::handles::dragged(frame, 7, to))
            );
            let grip = view.grip.take().expect("held");
            view.send(super::super::input::Input::Drag {
                from: grip.from,
                to: grip.to,
            });
        })
    });
    let resized = settle(&cx, |app| {
        view.read(app)
            .graphic
            .is_some_and(|now| now.2 < frame.2 && now.3 < frame.3)
    });
    assert!(resized, "the kernel resized the image");
}
