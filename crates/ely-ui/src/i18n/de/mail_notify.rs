//! German notification and attachment messages.

pub(super) const DE: &[(&str, &str)] = &[
    ("mail.notify.title", "Neue E-Mail"),
    ("mail.notify.body.one", "Von {senders}"),
    ("mail.notify.body", "{count} neue Nachrichten von {senders}"),
    ("mail.notify.more", "{senders} und andere"),
    ("mail.file.downloading", "Die Nachricht wird mit ihren Dateien heruntergeladen…"),
    ("mail.file.download_failed", "Die Dateien wurden nicht heruntergeladen: {error}"),
    ("mail.file.unread", "Die Datei konnte nicht gelesen werden: {error}"),
    ("mail.file.unsaved", "Die Datei wurde nicht gespeichert: {error}"),
    ("mail.file.saved", "{name} gespeichert"),
    ("mail.file.unopened", "Die Datei wurde nicht geöffnet: {error}"),
    ("mail.file.unsafe", "Diese Art von Datei kann ein Programm ausführen und wird daher hier nicht geöffnet. Speichern Sie sie und öffnen Sie sie selbst, wenn Sie dem Absender vertrauen."),
];
