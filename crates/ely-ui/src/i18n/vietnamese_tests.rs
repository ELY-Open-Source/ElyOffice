use super::{EN_MODULES, catalogs, resolve, vi};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn vietnamese_posix_tags_select_vietnamese() {
    for tag in ["vi", "vi-VN", "vi_VN.UTF-8", "VI-vn", "vi@tcvn"] {
        assert_eq!(resolve(tag), "vi-VN", "{tag}");
    }
    for tag in ["vie", "vix-VN"] {
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
fn vietnamese_translates_every_module_once_with_its_arguments() {
    for (english, vietnamese) in EN_MODULES.iter().zip(vi::VI) {
        let source: BTreeSet<_> = english.iter().map(|(key, _)| *key).collect();
        let translated: BTreeSet<_> = vietnamese.iter().map(|(key, _)| *key).collect();
        assert_eq!(source, translated, "a module's keys");
    }
    let [en, _, _, _, _, vietnamese, ..] = catalogs();
    let vi: BTreeMap<_, _> = vietnamese.iter().copied().collect();
    assert_eq!(vi.len(), vietnamese.len(), "duplicate Vietnamese key");
    for (key, english) in en {
        assert_eq!(arguments(english), arguments(vi[key]), "{key}");
        assert!(!vi[key].trim().is_empty(), "{key}");
    }
}

#[gpui::test]
fn vietnamese_catalog_renders_menus_counts_and_kernel_words(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("vi-VN", cx);
        assert_eq!(super::text(cx, "command.file.save", &[]), "Lưu");
        assert_eq!(super::text(cx, "table.sheet.default", &[("n", "1")]), "Trang tính1");
        assert_eq!(super::word_count(cx, 2, 3, None), "2 từ, 3 ký tự");
        assert_eq!(super::text(cx, "table.error.div_zero", &[]), "Lỗi: chia cho không");
        assert_eq!(super::text(cx, "table.style.accent1", &[]), "Trọng âm 1");
        assert_eq!(super::text(cx, "doc.paper.letter", &[]), "Lá thư");
        assert_eq!(super::text(cx, "tree.net.push", &[]), "Đẩy lên");
        assert_eq!(ely_gpui_component::i18n::text(cx, "find.replace_all", &[]), "Thay thế tất cả");
        let held = super::text(cx, "mail.error.outlook_held", &[("detail", "NO")]);
        for english in ["Settings", "Forwarding", "Let devices", "Recent activity", "This was me"] {
            assert!(!held.contains(english), "{english} in {held}");
        }
        assert!(held.contains("Chuyển tiếp và IMAP") && held.contains("“Đây là tôi”"), "{held}");
    });
}
