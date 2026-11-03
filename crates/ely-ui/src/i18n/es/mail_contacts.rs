//! Spanish contacts for ElyMail.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.boxes.contacts", "Contactos"),
    ("mail.contacts.name", "Nombre"),
    ("mail.contacts.address", "Dirección de correo"),
    ("mail.contacts.add", "Añadir contacto"),
    ("mail.contacts.remove", "Eliminar contacto"),
    ("mail.contacts.import", "Importar vCard…"),
    ("mail.contacts.export", "Exportar vCard…"),
    (
        "mail.contacts.empty",
        "Aún no hay contactos. Aquí se añaden las personas a las que escribe.",
    ),
    ("mail.contacts.unread", "No se pudieron leer los contactos: {error}"),
    ("mail.contacts.unsaved", "No se guardó el contacto: {error}"),
    (
        "mail.contacts.imported",
        "Importación: {added} añadidos, {renamed} con nombre cambiado, {unchanged} ya existentes; {skipped} tarjetas no tenían dirección",
    ),
    ("mail.contacts.unimported", "No se importó el archivo vCard: {error}"),
    ("mail.contacts.unexported", "No se exportaron los contactos: {error}"),
];
