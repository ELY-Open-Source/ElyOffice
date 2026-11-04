use super::{EN_MODULES, catalogs, de, resolve};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn german_regions_and_posix_tags_select_german() {
    for tag in ["de", "de-DE", "de_AT.UTF-8", "DE-ch", "de_DE@euro", "de.UTF-8"] {
        assert_eq!(resolve(tag), "de-DE", "{tag}");
    }
    for tag in ["deu", "dex-DE", "dsb-DE"] {
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
fn german_translates_every_module_once_with_its_arguments() {
    for (english, german) in EN_MODULES.iter().zip(de::DE) {
        let source: BTreeSet<_> = english.iter().map(|(key, _)| *key).collect();
        let translated: BTreeSet<_> = german.iter().map(|(key, _)| *key).collect();
        assert_eq!(source, translated, "a module's keys");
    }
    let [en, .., german] = catalogs();
    let de: BTreeMap<_, _> = german.iter().copied().collect();
    assert_eq!(de.len(), german.len(), "duplicate German key");
    for (key, english) in en {
        assert_eq!(arguments(english), arguments(de[key]), "{key}");
        assert!(!de[key].trim().is_empty(), "{key}");
    }
}

#[gpui::test]
fn german_catalog_renders_menus_counts_and_kernel_text(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("de-DE", cx);
        assert_eq!(super::text(cx, "command.file.save", &[]), "Speichern");
        assert_eq!(super::text(cx, "menu.edit", &[]), "Bearbeiten");
        assert_eq!(super::text(cx, "table.sheet.default", &[("n", "1")]), "Tabelle1");
        assert_eq!(super::text(cx, "doc.paper.letter", &[]), "Brief");
        assert_eq!(super::word_count(cx, 1, 1, None), "1 Wort, 1 Zeichen");
        assert_eq!(super::word_count(cx, 0, 0, None), "0 Wörter, 0 Zeichen");
        assert_eq!(super::word_count(cx, 2, 3, Some(1)), "1 von 2 Wörtern");
        assert_eq!(super::text(cx, "table.error.div_zero", &[]), "Fehler: Division durch Null");
        assert_eq!(super::text(cx, "table.style.accent1", &[]), "Betont 1");
        assert_eq!(super::text(cx, "table.figure.average", &[("value", "2")]), "Mittelwert 2");
        assert_eq!(super::text(cx, "show.layout.0", &[]), "Titelfolie");
        assert_eq!(super::text(cx, "show.transition.dissolve", &[]), "Auflösen");
        assert_eq!(super::text(cx, "tree.net.push", &[]), "Push");
        assert_eq!(ely_gpui_component::i18n::text(cx, "find.replace_all", &[]), "Alle ersetzen");
        assert_eq!(ely_gpui_component::i18n::text(cx, "dialog.cancel", &[]), "Abbrechen");
        let held = super::text(cx, "mail.error.outlook_held", &[("detail", "NO")]);
        for english in ["Settings", "Forwarding", "Let devices", "Recent activity", "This was me"] {
            assert!(!held.contains(english), "{english} in {held}");
        }
        assert!(held.contains("„Weiterleitung und IMAP“") && held.contains("„Das war ich“"), "{held}");
    });
}
