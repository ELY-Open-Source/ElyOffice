//! Files dropped on the page (#81).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{Entity, ExternalPaths, FileDropEvent, TestAppContext, px};

use super::{live_view_of, until};
use crate::LokView;
use crate::client::tests::{install, layout::png};

/// A window root that shows the view alone.
struct Page(Entity<LokView>);

impl gpui::Render for Page {
    fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        gpui::div().w(px(900.0)).h(px(700.0)).child(self.0.clone())
    }
}

/// Drops `paths` at `at`: the save, the files passed on.
fn dropped_on(name: &str, body: &str, at: (i32, i32), paths: &[PathBuf]) -> Option<(String, Vec<PathBuf>)> {
    let install = install()?;
    let mut cx = TestAppContext::single();
    cx.executor().allow_parking();
    cx.update(ely_gpui_component::init_for_tests);
    let (view, worker) = cx.update(|app| live_view_of(app, &install, name, body));
    let passed = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&passed);
    cx.update(|app| app.subscribe(&view, move |_, dropped: &crate::Dropped, _| seen.lock().expect("the record").extend(dropped.0.clone())).detach());
    let (_, vcx) = cx.add_window_view(|_, _| Page(view.clone()));
    vcx.run_until_parked();
    let position = vcx.update(|_, app| view.read(app).window_point(at));
    vcx.simulate_event(FileDropEvent::Entered { position, paths: ExternalPaths(paths.to_vec().into()) });
    vcx.simulate_event(FileDropEvent::Submit { position });
    vcx.run_until_parked();
    assert!(until(&cx, || cx.update(|app| view.read(app).settled())), "the drop never landed");
    let dir = tempfile::tempdir().expect("a temp dir");
    let flat = dir.path().join("flat");
    let doc = cx.update(|app| view.read(app).read_at());
    let kind = name.rsplit('.').next().expect("an extension");
    doc.save_as(&mut worker.lock().expect("the worker lock"), &crate::file_url(&flat), kind).expect("saves");
    let record = passed.lock().expect("the record").clone();
    Some((std::fs::read_to_string(&flat).expect("reads"), record))
}

fn path_of(url: String) -> PathBuf {
    crate::file_path(&url).expect("a file URL")
}

#[test]
fn pictures_dropped_on_a_page_go_in_where_they_fall_and_other_files_go_on() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let pictures = [png(dir.path(), "blue.png", [0, 0, 255, 255]), png(dir.path(), "red.PNG", [255, 0, 0, 255])].map(path_of);
    let other = dir.path().join("notes.docx");
    let paths = [pictures[0].clone(), other.clone(), pictures[1].clone()];
    // Below the text; the cursor began far above.
    let Some((flat, passed)) = dropped_on("three.fodt", crate::client::tests::reading::THREE, (1500, 6000), &paths) else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    assert_eq!(passed, [other], "only what is no picture goes on to the app");
    assert_eq!(flat.matches("<draw:image").count(), 2, "both pictures are in, neither replaced the other");
    let paragraph = |text: &str| flat.split("<text:p").find(|part| part.contains(text)).map(|part| part.contains("<draw:frame"));
    assert_eq!((paragraph(">one"), paragraph(">three")), (Some(false), Some(true)), "they hang on the paragraph they fell by");
}

#[test]
fn a_picture_dropped_on_a_sheet_or_a_slide_goes_in() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let picture = [path_of(png(dir.path(), "blue.png", [0, 0, 255, 255]))];
    let cases = [("book.fods", crate::client::tests::input::FODS), ("deck.fodp", crate::client::tests::show::DECK)];
    for (name, body) in cases {
        let Some((flat, passed)) = dropped_on(name, body, (3000, 3000), &picture) else {
            return eprintln!("SKIP: no pinned LOK build");
        };
        assert_eq!((flat.matches("<draw:image").count(), passed.len()), (1, 0), "{name}");
    }
}
