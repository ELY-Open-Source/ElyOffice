use super::{EN_MODULES, catalogs, es, resolve};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn spanish_regions_and_posix_tags_select_spanish() {
    for tag in [
        "es",
        "es-PE",
        "es_PE.UTF-8",
        "ES-mx",
        "es-419",
        "es_ES@euro",
        "es.UTF-8",
        "es@euro",
    ] {
        assert_eq!(resolve(tag), "es-ES", "{tag}");
    }
    for tag in ["est", "esx-PE", "zhx", "frx"] {
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
fn spanish_translates_every_module_once_with_its_arguments() {
    for (english, spanish) in EN_MODULES.iter().zip(es::ES) {
        let source: BTreeSet<_> = english.iter().map(|(key, _)| *key).collect();
        let translated: BTreeSet<_> = spanish.iter().map(|(key, _)| *key).collect();
        assert_eq!(source, translated, "a module's keys");
    }
    let [en, _, spanish, ..] = catalogs();
    let es: BTreeMap<_, _> = spanish.iter().copied().collect();
    assert_eq!(es.len(), spanish.len(), "duplicate Spanish key");
    for (key, english) in en {
        assert_eq!(arguments(english), arguments(es[key]), "{key}");
        assert!(!es[key].trim().is_empty(), "{key}");
    }
}

#[gpui::test]
fn spanish_catalog_renders_menus_counts_and_errors(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("es-ES", cx);
        assert_eq!(super::text(cx, "command.file.save", &[]), "Guardar");
        assert_eq!(super::text(cx, "menu.edit", &[]), "Editar");
        assert_eq!(super::text(cx, "menu.view", &[]), "Ver");
        assert_eq!(super::text(cx, "table.sheet.insert", &[]), "Insertar hoja");
        assert_eq!(super::text(cx, "settings.paper.letter", &[]), super::text(cx, "doc.paper.letter", &[]));
        assert_eq!(super::text(cx, "doc.paper.legal", &[]), "Legal");
        assert_eq!(super::word_count(cx, 1, 1, None), "1 palabra, 1 carácter");
        assert_eq!(super::word_count(cx, 2, 3, Some(1)), "1 de 2 palabras");
        assert_eq!(
            super::text(cx, "table.error.div_zero", &[]),
            "Error: división por cero"
        );
        assert_eq!(super::text(cx, "table.style.accent1", &[]), "Realce 1");
        assert_eq!(super::text(cx, "show.size.letter", &[]), "Carta");
        assert_eq!(
            ely_gpui_component::i18n::text(cx, "find.replace_all", &[]),
            "Reemplazar todo"
        );
        assert_eq!(
            ely_gpui_component::i18n::text(cx, "input.show_password", &[]),
            "Mostrar contraseña"
        );
    });
}
