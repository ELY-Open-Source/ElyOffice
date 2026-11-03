//! Spanish search of the mail held here, for ElyMail.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.search", "Buscar correo"),
    ("mail.search.in", "Buscar en"),
    ("mail.search.all", "En cualquier parte"),
    ("mail.search.from", "De"),
    ("mail.search.to", "Para"),
    ("mail.search.subject", "Asunto"),
    ("mail.search.body", "Texto"),
    ("mail.search.files", "Nombres de adjuntos"),
    ("mail.search.any_time", "Cualquier fecha"),
    (
        "mail.search.local",
        "Se buscó en el correo guardado en este equipo, en todas las cuentas.",
    ),
    (
        "mail.search.headers",
        "Se buscó en el correo guardado en este equipo, en todas las cuentas. De {count} solo están aquí los encabezados, así que no se buscó en su texto.",
    ),
    ("mail.search.none", "Ningún mensaje coincide"),
];
