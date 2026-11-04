//! German mail rule messages.

pub(super) const DE: &[(&str, &str)] = &[
    ("mail.rules.field", "Prüfen"),
    ("mail.rules.from", "Von"),
    ("mail.rules.to", "An oder Cc"),
    ("mail.rules.subject", "Betreff"),
    ("mail.rules.words", "Gesuchte Wörter"),
    ("mail.rules.action", "Dann"),
    ("mail.rules.flag", "markieren"),
    ("mail.rules.archive", "archivieren"),
    ("mail.rules.move", "nach {name} verschieben"),
    ("mail.rules.add", "Regel hinzufügen"),
    ("mail.rules.remove", "Regel entfernen"),
    ("mail.rules.said", "Neue E-Mail mit „{words}“ im Feld {field}: {action}"),
    ("mail.rules.unsaved", "Die Regel wurde nicht gespeichert: {error}"),
    ("mail.rules.unread", "Die Regeln konnten nicht gelesen werden: {error}"),
    ("mail.rules.refused", "Eine Regel konnte nicht ausgeführt werden: {error}"),
];
