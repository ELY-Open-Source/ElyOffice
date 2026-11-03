//! Spanish ElyMail writing: the compose window and the sender's identity.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.write.title", "Mensaje nuevo"),
    ("mail.write.from", "De"),
    ("mail.write.plain", "Texto sin formato"),
    ("mail.write.markdown", "Markdown"),
    ("mail.write.wrote", "El {date}, {sender} escribió:"),
    ("mail.write.wrote_undated", "{sender} escribió:"),
    (
        "mail.write.forwarded",
        "---------- Mensaje reenviado ----------",
    ),
    ("mail.write.lead.from", "De: {value}"),
    ("mail.write.lead.date", "Fecha: {value}"),
    ("mail.write.lead.subject", "Asunto: {value}"),
    ("mail.write.lead.to", "Para: {value}"),
    ("mail.write.lead.cc", "CC: {value}"),
    (
        "mail.write.missing",
        "{n} adjuntos no están descargados, así que no se reenvían",
    ),
    (
        "mail.write.headers_only",
        "Solo se conservan los encabezados de este mensaje, así que no se cita su texto",
    ),
    (
        "mail.write.sent_later",
        "En la Bandeja de salida. Los cambios posteriores no están incluidos; siguen aquí",
    ),
    (
        "mail.write.wait",
        "Espere a que el mensaje esté en la Bandeja de salida",
    ),
    ("mail.write.unnamed", "Archivo sin nombre"),
    (
        "mail.write.reading",
        "Leyendo {n} archivos; Enviar espera a que terminen",
    ),
    ("mail.write.no_account", "Añada una cuenta antes de escribir"),
    (
        "mail.write.link",
        "No se puede abrir este enlace de correo: {error}",
    ),
    (
        "mail.write.unread",
        "No se pudo empezar el mensaje: {error}",
    ),
    (
        "mail.write.unopened",
        "No se abrió la ventana del mensaje: {error}",
    ),
    (
        "mail.write.attach_failed",
        "No se adjuntó un archivo: {error}",
    ),
    ("mail.write.discard.title", "¿Eliminar este borrador?"),
    (
        "mail.write.discard.message",
        "Lo que ha escrito se elimina, aquí y en la carpeta Borradores del servidor.",
    ),
    ("mail.write.discard.go", "Eliminar"),
    ("mail.identity.name", "Nombre del remitente"),
    ("mail.identity.save", "Guardar nombre y firma"),
    ("mail.identity.saved", "Se guardaron el nombre y la firma"),
    (
        "mail.identity.unsaved",
        "No se guardaron el nombre y la firma: {error}",
    ),
    (
        "mail.identity.unread",
        "No se pudieron leer el nombre y la firma: {error}",
    ),
];
