//! Spanish message files and mbox files for ElyMail.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.boxes.imported", "{name} (en este equipo)"),
    ("mail.eml.save", "Guardar como EML…"),
    ("mail.eml.title", "Archivo de mensaje"),
    ("mail.eml.unread", "No se pudo leer el archivo de mensaje"),
    ("mail.eml.unopened", "No se abrió el archivo de mensaje: {error}"),
    ("mail.eml.unsaved", "No se guardó el mensaje: {error}"),
    ("mail.eml.saved", "Guardado en {path}"),
    (
        "mail.eml.incomplete",
        "Este mensaje aún no está completo. Se está descargando; vuelva a guardarlo cuando termine.",
    ),
    (
        "mail.eml.pictures",
        "En un archivo de mensaje no se cargan imágenes web; este tiene {n}.",
    ),
    ("mail.mbox.import", "Importar MBOX…"),
    ("mail.mbox.importing", "Importando «{name}»…"),
    (
        "mail.mbox.imported",
        "Se importaron {n} mensajes en «{name}» en este equipo",
    ),
    (
        "mail.mbox.imported.one",
        "Se importó 1 mensaje en «{name}» en este equipo",
    ),
    ("mail.mbox.unimported", "No se importó el archivo mbox: {error}"),
    ("mail.mbox.export", "Exportar como MBOX…"),
    ("mail.mbox.unexported", "No se exportó la carpeta: {error}"),
    (
        "mail.mbox.partial",
        "Se omitieron {n} mensajes: solo están aquí sus encabezados, sus archivos no se descargaron o contienen partes binarias sin procesar que mbox alteraría.",
    ),
    (
        "mail.mbox.partial.one",
        "Se omitió 1 mensaje: solo están aquí sus encabezados, sus archivos no se descargaron o contiene partes binarias sin procesar que mbox alteraría.",
    ),
    ("mail.mbox.remove", "Eliminar carpeta importada…"),
    ("mail.mbox.remove_title", "¿Eliminar «{name}»?"),
    (
        "mail.mbox.remove_message",
        "Sus mensajes se eliminan de este equipo. Ningún servidor los conserva.",
    ),
    ("mail.mbox.remove_go", "Eliminar"),
    ("mail.mbox.unremoved", "No se eliminó la carpeta: {error}"),
];
