//! Ely's own errors read in the window's language (R15.38).

use ely_office::prelude::*;
use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Blank, docs, settle, with_window};
use crate::docs::Docs;
use crate::files::Template;
use crate::merge::{Table, read_csv};

/// A window root that keeps Docs' toasts.
pub(super) fn rooted(cx: &mut TestAppContext, docs: &Entity<Docs>) -> Entity<ely_ui::Root> {
    // Toasts read the theme when they time out.
    cx.update(ely_gpui_component::init_for_tests);
    let window = cx.add_window(|_, _| Blank);
    let shown = VisualTestContext::from_window(window.into(), cx).into_mut();
    shown.update(|window, app| {
        let root = ely_ui::document_root(ely_platform::AppId::Doc, docs.clone().into(), &ely_ui::commands::FILE, Vec::new(), window, app);
        docs.update(app, |docs, _| docs.core.root = root.downgrade());
        root
    })
}

#[test]
fn a_merge_whose_source_closed_says_so_in_chinese_and_spanish() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let root = rooted(&mut cx, &docs);
    let table = Table { name: "people.csv".into(), headers: vec!["Name".into()], rows: vec![vec!["Ada".into()]] };
    for (locale, said) in [("zh-CN", "文档已关闭"), ("es-ES", "el documento se cerró")] {
        cx.update(|app| ely_ui::i18n::install_as(locale, app));
        let (table, folder) = (table.clone(), dir.path().to_path_buf());
        // Tab 99 never opened: the source is gone.
        with_window(&mut cx, &docs, move |docs, _, cx| docs.generate(99, table, false, folder, cx));
        let toasts = cx.update(|app| root.read(app).toasts(app));
        assert!(toasts.iter().any(|(text, _)| text.contains(said)), "{locale}: {toasts:?}");
    }
}

#[gpui::test]
fn a_csv_without_rows_says_so_in_chinese_and_spanish(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let path = dir.path().join("people.csv");
    std::fs::write(&path, "Name,Name\na,b\n").expect("writes");
    let twice = read_csv(&path).expect_err("two columns");
    std::fs::write(&path, "Name,City\n").expect("writes");
    let empty = read_csv(&path).expect_err("no rows");
    cx.update(|app| {
        ely_ui::settings::install_defaults(app);
        for (locale, named, none) in [
            ("zh-CN", "有两列都叫 Name", "文件里没有数据行"),
            ("es-ES", "dos columnas se llaman Name", "el archivo no tiene filas"),
        ] {
            ely_ui::i18n::install_as(locale, app);
            let said = |error| ely_ui::i18n::error(app, error);
            assert_eq!((said(&twice), said(&empty)), (named.into(), none.into()), "{locale}");
        }
    });
}

#[test]
fn a_save_as_target_another_tab_took_meanwhile_says_so_in_chinese_and_spanish() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let root = rooted(&mut cx, &docs);
    let fodt = cx.update(|app| Template::Report.fodt(app));
    let [a, b, target] = ["a.fodt", "b.fodt", "copy.odt"].map(|name| dir.path().join(name));
    for path in [&a, &b, &target] {
        std::fs::write(path, &fodt).expect("writes");
    }
    for path in [&a, &b] {
        let path = path.clone();
        with_window(&mut cx, &docs, move |docs, _, cx| docs.open_file(path, cx));
    }
    settle(&cx, |app| docs.read(app).core.tabs.len() == 2 && docs.read(app).core.loading.is_empty());
    let (first, second) = cx.update(|app| (docs.read(app).core.tabs[0].id, docs.read(app).core.tabs[1].id));
    for (locale, said) in [("zh-CN", "copy.odt 已在另一个标签页打开"), ("es-ES", "copy.odt está abierto en otra pestaña")] {
        cx.update(|app| ely_ui::i18n::install_as(locale, app));
        let (b, target) = (b.clone(), target.clone());
        with_window(&mut cx, &docs, move |docs, window, cx| {
            take(docs, second, b);
            docs.write_as(first, target.clone(), None, window, cx);
            // Before the export lands, the other tab takes the target.
            take(docs, second, target);
        });
        settle(&cx, |app| root.read(app).toasts(app).iter().any(|(text, _)| text.contains(said)));
    }
}

/// Tab `id` holds `path`, registered as an open does.
fn take(docs: &mut Docs, id: u64, path: std::path::PathBuf) {
    if let Some(old) = docs.tab(id).and_then(|tab| tab.registry) {
        docs.core.sessions.close(old);
    }
    let Ok(ely_files::Open::Fresh(registry)) = docs.core.sessions.open(&path, false) else {
        panic!("{} is free", path.display())
    };
    let tab = docs.tab_mut(id).expect("the tab");
    (tab.registry, tab.session) = (Some(registry), ely_files::Session::load(path).expect("loads"));
}

#[test]
fn a_bad_csv_logs_its_key_never_its_columns_or_words() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut cx = TestAppContext::single();
    cx.update(ely_gpui_component::init_for_tests);
    let Some(docs) = docs(&mut cx, dir.path()) else {
        return eprintln!("SKIP: no LOK build");
    };
    let root = rooted(&mut cx, &docs);
    let csv = dir.path().join("people.csv");
    std::fs::write(&csv, "ZqxPatient42,ZqxPatient42\nAda,Lin\n").expect("writes");
    cx.update(|app| ely_ui::i18n::install_as("zh-CN", app));
    with_window(&mut cx, &docs, |docs, _, cx| docs.take_csv(0, &csv, cx));
    let said = cx.update(|app| ely_ui::i18n::text(app, "doc.merge.twice", &[("name", "ZqxPatient42")]));
    let toasts = cx.update(|app| root.read(app).toasts(app));
    assert!(toasts.iter().any(|(text, _)| text.contains(said.as_ref())), "{toasts:?}");
    let kept = ely_ui::logs::kept();
    assert!(kept.iter().any(|line| line.ends_with("doc: the merge data did not read: doc.merge.twice")), "{kept:?}");
    assert!(!kept.iter().any(|line| line.contains("ZqxPatient42") || line.contains(said.as_ref())), "{kept:?}");
}
