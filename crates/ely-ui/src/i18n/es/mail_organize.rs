//! Spanish ElyMail read, flag, move and delete messages.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.act.read", "Marcar como leído"),
    ("mail.act.unread", "Marcar como no leído"),
    ("mail.act.flag", "Marcar"),
    ("mail.act.unflag", "Quitar marca"),
    ("mail.act.archive", "Archivar"),
    ("mail.act.delete", "Eliminar"),
    ("mail.act.restore", "Mover a la Bandeja de entrada"),
    ("mail.act.move", "Mover a"),
    ("mail.organize.failed", "Sin cambios: {error}"),
    (
        "mail.organize.dropped",
        "{n} cambios no se aplicaron en el servidor y se han deshecho: {reason}",
    ),
    (
        "mail.organize.dropped.one",
        "Un cambio no se aplicó en el servidor y se ha deshecho: {reason}",
    ),
    ("mail.organize.gone", "el mensaje ya no está en el servidor"),
    ("mail.organize.renumbered", "la carpeta se reconstruyó en el servidor"),
    ("mail.organize.unsupported", "el servidor no admite {name}"),
    ("mail.organize.unplaced", "el servidor movió el mensaje sin indicar adónde"),
    ("mail.organize.vanished", "la carpeta {name} ya no está en el servidor"),
    ("mail.erase.title", "¿Eliminar {n} mensajes definitivamente?"),
    ("mail.erase.title.one", "¿Eliminar este mensaje definitivamente?"),
    ("mail.erase.message", "No se podrán restaurar, ni aquí ni en el servidor."),
    ("mail.erase.go", "Eliminar definitivamente"),
    ("mail.sync.waiting", "Cambios"),
    ("mail.sync.waiting_value", "{n} cambios hechos aquí esperan al servidor"),
    ("mail.sync.waiting_value.one", "1 cambio hecho aquí espera al servidor"),
    ("mail.sync.waiting_none", "Todos los cambios están en el servidor"),
];
