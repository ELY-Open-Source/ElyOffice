//! Ely's own errors read in the window's language (R15.38).

use gpui::{Entity, TestAppContext, VisualTestContext};

use super::{Blank, docs, with_window};
use crate::docs::Docs;
use crate::merge::{Table, read_csv};

/// A window root that keeps Docs' toasts.
fn rooted(cx: &mut TestAppContext, docs: &Entity<Docs>) -> Entity<ely_ui::Root> {
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
