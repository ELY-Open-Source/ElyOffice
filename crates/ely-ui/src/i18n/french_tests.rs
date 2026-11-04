use super::{EN_MODULES, catalogs, fr, resolve};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn french_regions_and_posix_tags_select_french() {
    for tag in ["fr", "fr-FR", "fr_CA.UTF-8", "FR-be", "fr_CH@euro", "fr.UTF-8"] {
        assert_eq!(resolve(tag), "fr-FR", "{tag}");
    }
    for tag in ["fra", "frx-FR", "fy-NL"] {
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
fn french_translates_every_module_once_with_its_arguments() {
    for (english, french) in EN_MODULES.iter().zip(fr::FR) {
        let source: BTreeSet<_> = english.iter().map(|(key, _)| *key).collect();
        let translated: BTreeSet<_> = french.iter().map(|(key, _)| *key).collect();
        assert_eq!(source, translated, "a module's keys");
    }
    let [en, _, _, _, _, _, french, ..] = catalogs();
    let fr: BTreeMap<_, _> = french.iter().copied().collect();
    assert_eq!(fr.len(), french.len(), "duplicate French key");
    for (key, english) in en {
        assert_eq!(arguments(english), arguments(fr[key]), "{key}");
        assert!(!fr[key].trim().is_empty(), "{key}");
    }
}

#[gpui::test]
fn french_catalog_renders_menus_counts_and_kernel_text(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("fr-FR", cx);
        assert_eq!(super::text(cx, "command.file.save", &[]), "Enregistrer");
        assert_eq!(super::text(cx, "menu.edit", &[]), "Édition");
        assert_eq!(super::text(cx, "table.sheet.insert", &[]), "Insérer une feuille");
        assert_eq!(super::text(cx, "settings.paper.letter", &[]), super::text(cx, "doc.paper.letter", &[]));
        assert_eq!(super::text(cx, "doc.paper.legal", &[]), "Légal");
        assert_eq!(super::word_count(cx, 1, 1, None), "1 mot, 1 caractère");
        assert_eq!(super::word_count(cx, 2, 3, Some(1)), "1 sur 2 mots");
        assert_eq!(super::text(cx, "table.error.div_zero", &[]), "Erreur : division par zéro");
        assert_eq!(super::text(cx, "table.style.good", &[]), "Correct");
        assert_eq!(super::text(cx, "show.layout.0", &[]), "Diapo de titre");
        assert_eq!(super::text(cx, "tree.net.push", &[]), "Pousser");
        assert_eq!(ely_gpui_component::i18n::text(cx, "find.replace_all", &[]), "Tout remplacer");
        assert_eq!(ely_gpui_component::i18n::text(cx, "dialog.cancel", &[]), "Annuler");
    });
}
