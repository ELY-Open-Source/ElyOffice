use super::{EN_MODULES, catalogs, ja, resolve};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn japanese_and_posix_tags_select_japanese() {
    for tag in ["ja", "ja-JP", "ja_JP.UTF-8", "JA-jp", "ja_JP.eucJP", "ja_JP.SJIS", "ja@euro"] {
        assert_eq!(resolve(tag), "ja-JP", "{tag}");
    }
    for tag in ["jav", "jax-JP", "jp", "ka-GE"] {
        assert_eq!(resolve(tag), "en-US", "{tag}");
    }
}

fn arguments(message: &str) -> BTreeSet<&str> {
    message
        .split('{')
        .skip(1)
        .map(|part| part.split_once('}').expect("closed argument").0)
        .collect()
}

#[test]
fn japanese_translates_every_module_once_with_its_arguments() {
    for (english, japanese) in EN_MODULES.iter().zip(ja::JA) {
        let source: BTreeSet<_> = english.iter().map(|(key, _)| *key).collect();
        let translated: BTreeSet<_> = japanese.iter().map(|(key, _)| *key).collect();
        assert_eq!(source, translated, "a module's keys");
    }
    let [en, _, _, japanese, ..] = catalogs();
    let ja: BTreeMap<_, _> = japanese.iter().copied().collect();
    assert_eq!(ja.len(), japanese.len(), "duplicate Japanese key");
    for (key, english) in en {
        assert_eq!(arguments(english), arguments(ja[key]), "{key}");
        assert!(!ja[key].trim().is_empty(), "{key}");
    }
}

#[gpui::test]
fn japanese_catalog_renders_menus_counts_and_kernel_text(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("ja-JP", cx);
        assert_eq!(super::text(cx, "command.file.save_as", &[]), "名前を付けて保存…");
        assert_eq!(super::text(cx, "menu.edit", &[]), "編集");
        assert_eq!(super::text(cx, "settings.paper.letter", &[]), super::text(cx, "doc.paper.letter", &[]));
        assert_eq!(super::word_count(cx, 1, 1, None), "1 語、1 文字");
        assert_eq!(super::word_count(cx, 2, 3, Some(1)), "2 語中 1 語");
        assert_eq!(super::text(cx, "table.error.div_zero", &[]), "エラー:ゼロによる除算");
        assert_eq!(super::text(cx, "table.style.accent1", &[]), "アクセント1");
        assert_eq!(super::text(cx, "table.figure.sum", &[("value", "6")]), "合計 6");
        assert_eq!(super::text(cx, "show.layout.19", &[]), "タイトルのみ");
        assert_eq!(super::text(cx, "show.effect.start.click", &[]), "クリック時");
        assert_eq!(super::text(cx, "mark.block.code", &[]), "コードブロック");
        assert_eq!(ely_gpui_component::i18n::text(cx, "find.replace_all", &[]), "すべて置換");
    });
}
