//! Spanish local rules for ElyMail.

pub(super) const ES: &[(&str, &str)] = &[
    ("mail.rules.field", "Buscar en"),
    ("mail.rules.from", "De"),
    ("mail.rules.to", "Para o CC"),
    ("mail.rules.subject", "Asunto"),
    ("mail.rules.words", "Palabras que buscar"),
    ("mail.rules.action", "Entonces"),
    ("mail.rules.flag", "marcarlo"),
    ("mail.rules.archive", "archivarlo"),
    ("mail.rules.move", "moverlo a {name}"),
    ("mail.rules.add", "Añadir regla"),
    ("mail.rules.remove", "Eliminar regla"),
    (
        "mail.rules.said",
        "Correo nuevo cuyo campo {field} contiene «{words}»: {action}",
    ),
    ("mail.rules.unsaved", "No se guardó la regla: {error}"),
    ("mail.rules.unread", "No se pudieron leer las reglas: {error}"),
    ("mail.rules.refused", "Una regla no pudo actuar: {error}"),
];
