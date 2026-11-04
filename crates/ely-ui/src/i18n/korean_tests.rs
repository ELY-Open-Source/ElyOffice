use super::{EN_MODULES, catalogs, ko, resolve};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn korean_and_posix_tags_select_korean() {
    for tag in ["ko", "ko-KR", "ko_KR.UTF-8", "KO-kr", "ko_KR.eucKR", "ko.UTF-8"] {
        assert_eq!(resolve(tag), "ko-KR", "{tag}");
    }
    for tag in ["kok", "kor-KR", "kn-IN"] {
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
fn korean_translates_every_module_once_with_its_arguments() {
    for (english, korean) in EN_MODULES.iter().zip(ko::KO) {
        let source: BTreeSet<_> = english.iter().map(|(key, _)| *key).collect();
        let translated: BTreeSet<_> = korean.iter().map(|(key, _)| *key).collect();
        assert_eq!(source, translated, "a module's keys");
    }
    let [en, _, _, _, korean, ..] = catalogs();
    let ko: BTreeMap<_, _> = korean.iter().copied().collect();
    assert_eq!(ko.len(), korean.len(), "duplicate Korean key");
    for (key, english) in en {
        assert_eq!(arguments(english), arguments(ko[key]), "{key}");
        assert!(!ko[key].trim().is_empty(), "{key}");
    }
}

#[gpui::test]
fn korean_catalog_renders_menus_counts_and_errors(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("ko-KR", cx);
        assert_eq!(super::text(cx, "command.file.save", &[]), "저장");
        assert_eq!(super::text(cx, "menu.edit", &[]), "편집");
        assert_eq!(super::text(cx, "table.sheet.insert", &[]), "시트 삽입");
        assert_eq!(super::text(cx, "settings.paper.letter", &[]), super::text(cx, "doc.paper.letter", &[]));
        assert_eq!(super::word_count(cx, 1, 1, None), "단어 1개, 문자 1개");
        assert_eq!(super::word_count(cx, 2, 3, Some(1)), "단어 2개 중 1개");
        assert_eq!(super::text(cx, "table.error.div_zero", &[]), "오류: 영(0)으로 나눔");
        assert_eq!(super::text(cx, "table.style.warning", &[]), "경고");
        assert_eq!(super::text(cx, "show.layout.19", &[]), "제목만");
        assert_eq!(super::text(cx, "show.layout.16", &[]), "제목, 위에 내용 2개, 아래에 내용");
        assert_eq!(super::text(cx, "mark.block.link_reference", &[]), "링크 참조");
        assert_eq!(super::text(cx, "mark.kind.html", &[]), "HTML 블록");
        assert_eq!(super::text(cx, "tree.worktree.bare", &[]), "간략한 저장소");
        assert_eq!(super::text(cx, "mail.contacts.empty", &[]), "아직 연락처가 없습니다. 메일을 보내는 상대가 여기에 추가됩니다.");
        assert_eq!(ely_gpui_component::i18n::text(cx, "find.replace_all", &[]), "모두 바꾸기");
        assert_eq!(ely_gpui_component::i18n::text(cx, "input.show_password", &[]), "암호 표시");
    });
}
