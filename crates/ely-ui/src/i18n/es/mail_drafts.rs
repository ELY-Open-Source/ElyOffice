//! Spanish ElyMail drafts, here and on the server.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.boxes.drafts", "Borradores"),
    ("mail.drafts.pick", "Elija un borrador para verlo"),
    ("mail.drafts.to", "Para"),
    ("mail.drafts.cc", "CC"),
    ("mail.drafts.bcc", "CCO"),
    ("mail.drafts.saved", "Guardado"),
    ("mail.drafts.continue", "Seguir escribiendo"),
    ("mail.drafts.delete", "Eliminar borrador…"),
    ("mail.drafts.kept", "El mensaje se guarda en Borradores"),
    ("mail.drafts.unkept", "No se guardó el borrador: {error}"),
    (
        "mail.drafts.undropped",
        "No se eliminó el borrador: {error}",
    ),
    (
        "mail.drafts.unread",
        "No se pudieron leer los borradores: {error}",
    ),
    (
        "mail.drafts.up",
        "Guardar también los borradores en la carpeta Borradores del servidor",
    ),
    (
        "mail.drafts.unsaved_choice",
        "No se guardó la opción de borradores: {error}",
    ),
    (
        "mail.drafts.no_folder",
        "Guardado en este equipo. El servidor todavía no tiene una carpeta Borradores.",
    ),
    (
        "mail.drafts.server_failed",
        "Guardado en este equipo. No se guardó la copia del servidor: {error}",
    ),
    (
        "mail.drafts.server_stale",
        "Puede quedar una copia antigua en la carpeta Borradores del servidor: {error}",
    ),
];
