//! German contacts messages.

pub(super) const DE: &[(&str, &str)] = &[
    ("mail.boxes.contacts", "Kontakte"),
    ("mail.contacts.name", "Name"),
    ("mail.contacts.address", "E-Mail-Adresse"),
    ("mail.contacts.add", "Kontakt hinzufügen"),
    ("mail.contacts.remove", "Kontakt entfernen"),
    ("mail.contacts.import", "vCard importieren…"),
    ("mail.contacts.export", "vCard exportieren…"),
    ("mail.contacts.empty", "Noch keine Kontakte. Personen, denen Sie schreiben, werden hier hinzugefügt."),
    ("mail.contacts.unread", "Die Kontakte konnten nicht gelesen werden: {error}"),
    ("mail.contacts.unsaved", "Der Kontakt wurde nicht gespeichert: {error}"),
    ("mail.contacts.imported", "Importiert: {added} hinzugefügt, {renamed} umbenannt, {unchanged} schon vorhanden; {skipped} Karten hatten keine Adresse"),
    ("mail.contacts.unimported", "Die vCard-Datei wurde nicht importiert: {error}"),
    ("mail.contacts.unexported", "Die Kontakte wurden nicht exportiert: {error}"),
];
