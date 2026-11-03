//! Spanish rows on the settings storage page for ElyMail.

pub(super) const ES: &[(&str, &str)] = &[
    (
        "mail.storage.unread",
        "No se pudo leer el almacenamiento del correo: {error}",
    ),
    ("mail.storage.index", "Índice de correo"),
    (
        "mail.storage.index_hint",
        "Cuentas, carpetas, encabezados, borradores y la Bandeja de salida: {size}; búsqueda: {search}",
    ),
    ("mail.storage.originals", "Correo descargado"),
    (
        "mail.storage.originals_hint",
        "{size}. Al limpiar se libera el correo que un servidor aún conserva; se vuelve a descargar al sincronizar o al abrirlo. Las carpetas sin conexión, POP3 y el correo importado se conservan.",
    ),
    ("mail.storage.forget", "Limpiar…"),
    ("mail.storage.forget_title", "¿Limpiar el correo descargado?"),
    (
        "mail.storage.forget_message",
        "Los cuerpos y archivos del correo de servidores IMAP se eliminan de este equipo; los encabezados se conservan. El correo reciente se vuelve a descargar en la próxima sincronización, y el más antiguo, al abrirlo.",
    ),
    ("mail.storage.forget_go", "Limpiar"),
    ("mail.storage.forgot", "{n} mensajes liberados"),
    ("mail.storage.forgot.one", "1 mensaje liberado"),
    ("mail.storage.unforgotten", "No se limpió el correo: {error}"),
    ("mail.storage.cap", "Límite del correo descargado"),
    ("mail.storage.cap_none", "Sin límite"),
    (
        "mail.storage.cap_hint",
        "Al superar el límite, las descargas más antiguas vuelven a quedarse solo con sus encabezados. Las carpetas sin conexión y el correo importado se conservan.",
    ),
    (
        "mail.storage.cap_unread",
        "No se puede leer el límite guardado, así que no se aplica ninguno: {error}",
    ),
    ("mail.storage.cap_unsaved", "No se guardó el límite: {error}"),
    (
        "mail.storage.untrimmed",
        "Las descargas no se recortaron hasta el límite: {error}",
    ),
    ("mail.storage.all", "Exportar todo el correo"),
    (
        "mail.storage.all_hint",
        "Cada carpeta se convierte en un archivo mbox, dentro de una carpeta por cuenta.",
    ),
    ("mail.storage.export", "Exportar…"),
    (
        "mail.storage.exported",
        "{n} carpetas exportadas; {left} mensajes omitidos (incompletos aquí o binarios sin procesar)",
    ),
    (
        "mail.storage.exported.one",
        "1 carpeta exportada; {left} mensajes omitidos (incompletos aquí o binarios sin procesar)",
    ),
];
