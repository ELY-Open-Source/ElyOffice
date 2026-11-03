//! Spanish notices of new mail, and its files, for ElyMail.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.notify.title", "Correo nuevo"),
    ("mail.notify.body.one", "De {senders}"),
    ("mail.notify.body", "{count} mensajes nuevos de {senders}"),
    ("mail.notify.more", "{senders} y otros"),
    ("mail.file.downloading", "Descargando el mensaje con sus adjuntos…"),
    ("mail.file.download_failed", "No se descargaron los adjuntos: {error}"),
    ("mail.file.unread", "No se pudo leer el archivo: {error}"),
    ("mail.file.unsaved", "No se guardó el archivo: {error}"),
    ("mail.file.saved", "Se guardó {name}"),
    ("mail.file.unopened", "No se abrió el archivo: {error}"),
    (
        "mail.file.unsafe",
        "Este tipo de archivo puede ejecutar un programa, por lo que no se abre aquí. Guárdelo y ábralo usted mismo si confía en el remitente.",
    ),
];
