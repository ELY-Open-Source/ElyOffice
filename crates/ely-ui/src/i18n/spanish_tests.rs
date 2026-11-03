use super::{catalogs, es, resolve, spanish_catalog, spanish_key};
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
    ] {
        assert_eq!(resolve(tag), "es-ES", "{tag}");
    }
    for tag in ["est", "esx-PE", "zhx", "fr-FR"] {
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
fn spanish_covers_doc_table_and_shared_messages_without_duplicates() {
    let (en, _) = catalogs();
    let source: BTreeMap<_, _> = en
        .iter()
        .filter(|(key, _)| spanish_key(key))
        .copied()
        .collect();
    let translated = es::catalog();
    let es: BTreeMap<_, _> = translated.iter().copied().collect();
    assert_eq!(es.len(), translated.len(), "duplicate Spanish key");
    assert_eq!(
        source.keys().collect::<Vec<_>>(),
        es.keys().collect::<Vec<_>>()
    );
    for (key, english) in source {
        assert_eq!(arguments(english), arguments(es[key]), "{key}");
        assert!(!es[key].is_empty(), "{key}");
    }
    let complete = spanish_catalog(&en);
    assert_eq!(en.len(), complete.len());
    assert_eq!(
        en.iter().map(|(key, _)| key).collect::<BTreeSet<_>>(),
        complete.iter().map(|(key, _)| key).collect::<BTreeSet<_>>()
    );
}

#[gpui::test]
fn spanish_catalog_renders_menus_counts_and_errors(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        super::install_as("es-ES", cx);
        assert_eq!(super::text(cx, "command.file.save", &[]), "Guardar");
        assert_eq!(super::text(cx, "menu.edit", &[]), "Editar");
        assert_eq!(super::text(cx, "menu.view", &[]), "Ver");
        assert_eq!(super::text(cx, "table.sheet.insert", &[]), "Insertar hoja");
        assert_eq!(super::word_count(cx, 1, 1, None), "1 palabra, 1 carácter");
        assert_eq!(super::word_count(cx, 2, 3, Some(1)), "1 de 2 palabras");
        assert_eq!(
            super::text(cx, "table.error.div_zero", &[]),
            "Error: división por cero"
        );
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
