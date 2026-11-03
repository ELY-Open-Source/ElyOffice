//! Spanish ElyMail outbox and how sends end.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.boxes.outbox", "Bandeja de salida"),
    (
        "mail.outbox.pick",
        "Elija un mensaje para ver en qué estado está",
    ),
    ("mail.outbox.state", "Estado"),
    ("mail.outbox.queued_at", "En cola desde"),
    ("mail.outbox.tried_at", "Último intento"),
    ("mail.outbox.message_id", "Message-ID"),
    ("mail.outbox.queued", "En cola"),
    ("mail.outbox.waiting", "Esperando al servidor: {error}"),
    ("mail.outbox.sending", "Enviando"),
    (
        "mail.outbox.accepted",
        "Aceptado por el servidor; se está guardando su copia en Enviados",
    ),
    (
        "mail.outbox.unfiled",
        "Aceptado por el servidor; su copia no se guardó en Enviados: {error}",
    ),
    (
        "mail.outbox.filing_later",
        "Aceptado por el servidor; su copia para Enviados espera al servidor: {error}. Se volverá a intentar en la próxima sincronización.",
    ),
    ("mail.outbox.failed", "Falló: {error}"),
    ("mail.outbox.unsure", "Resultado desconocido: {error}"),
    (
        "mail.outbox.stopped",
        "Resultado desconocido: ElyMail se detuvo mientras lo enviaba",
    ),
    ("mail.outbox.cancelled", "Cancelado"),
    (
        "mail.outbox.unsure_help",
        "Puede que el servidor tenga este mensaje. ElyMail no lo vuelve a enviar por sí solo. Compruebe Enviados o pregunte a un destinatario y, después, quítelo de aquí o vuelva a enviarlo.",
    ),
    (
        "mail.outbox.accepted_help",
        "Aceptado significa que el servidor lo recibió, no que se haya entregado ni leído.",
    ),
    ("mail.outbox.retry", "Reintentar"),
    ("mail.outbox.send", "Enviar"),
    ("mail.outbox.send_again", "Volver a enviar de todos modos"),
    ("mail.outbox.cancel", "Cancelar envío"),
    ("mail.outbox.edit", "Editar"),
    ("mail.outbox.forget", "Quitar"),
    ("mail.outbox.file_again", "Volver a guardar la copia"),
    ("mail.outbox.sent", "El servidor aceptó el mensaje"),
    (
        "mail.outbox.offline",
        "No se pudo contactar con el servidor. El mensaje espera en la Bandeja de salida y se volverá a intentar en la próxima sincronización o cuando elija Reintentar allí. Allí también puede cancelarlo.",
    ),
    (
        "mail.outbox.failed_toast",
        "No enviado: {error}. El mensaje permanece en la Bandeja de salida.",
    ),
    (
        "mail.outbox.unsure_toast",
        "Puede que el servidor tenga el mensaje, pero no lo confirmó: {error}. No se vuelve a enviar por sí solo; consulte la Bandeja de salida.",
    ),
    (
        "mail.outbox.unfiled_toast",
        "Enviado, pero su copia no se guardó en Enviados: {error}. Solo se volverá a intentar la copia.",
    ),
    (
        "mail.outbox.unfiled_later",
        "Enviado, pero no se pudo contactar con el servidor para su copia en Enviados: {error}. La copia se volverá a intentar en la próxima sincronización; el mensaje no se vuelve a enviar.",
    ),
    (
        "mail.outbox.no_sent_folder",
        "la cuenta no tiene una carpeta Enviados",
    ),
    (
        "mail.outbox.unread",
        "No se pudo leer la Bandeja de salida: {error}",
    ),
    (
        "mail.outbox.unsaved",
        "No se actualizó la Bandeja de salida: {error}",
    ),
    (
        "mail.outbox.unqueued",
        "El mensaje no se puso en la Bandeja de salida: {error}",
    ),
    ("mail.outbox.copy", "Guardar una copia en Enviados"),
    (
        "mail.outbox.copy_note",
        "Desactivado de forma predeterminada aquí: este proveedor guarda por sí mismo el correo enviado.",
    ),
    (
        "mail.outbox.unsaved_choice",
        "No se guardó la opción de copia en Enviados: {error}",
    ),
    ("mail.outbox.files_size", "Adjuntos: {size}"),
    (
        "mail.outbox.too_large",
        "Unos {size} en total; el servidor de {address} admite como máximo {limit}",
    ),
    ("mail.outbox.quit.title", "¿Salir con correo sin enviar?"),
    (
        "mail.outbox.quit.message",
        "{n} mensajes de la Bandeja de salida aún no han terminado. ElyMail no los envía después de cerrarse; lo vuelve a intentar la próxima vez que se abra.",
    ),
    (
        "mail.outbox.quit.message.one",
        "1 mensaje de la Bandeja de salida aún no ha terminado. ElyMail no lo envía después de cerrarse; lo vuelve a intentar la próxima vez que se abra.",
    ),
    (
        "mail.outbox.quit.sending",
        "Uno se está enviando ahora mismo; si sale, puede que no llegue a saberse cómo terminó.",
    ),
    ("mail.outbox.quit.go", "Salir"),
];
