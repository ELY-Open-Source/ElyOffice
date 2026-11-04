//! French contacts messages.

pub(super) const FR: &[(&str, &str)] = &[
    ("mail.boxes.contacts", "Contacts"),
    ("mail.contacts.name", "Nom"),
    ("mail.contacts.address", "Adresse électronique"),
    ("mail.contacts.add", "Ajouter un contact"),
    ("mail.contacts.remove", "Supprimer le contact"),
    ("mail.contacts.import", "Importer une vCard…"),
    ("mail.contacts.export", "Exporter en vCard…"),
    ("mail.contacts.empty", "Aucun contact pour l’instant. Les personnes à qui vous écrivez sont ajoutées ici."),
    ("mail.contacts.unread", "Les contacts n’ont pas pu être lus : {error}"),
    ("mail.contacts.unsaved", "Le contact n’a pas été enregistré : {error}"),
    ("mail.contacts.imported", "Importés : {added} ajoutés, {renamed} renommés, {unchanged} déjà présents ; {skipped} cartes sans adresse"),
    ("mail.contacts.unimported", "Le fichier vCard n’a pas été importé : {error}"),
    ("mail.contacts.unexported", "Les contacts n’ont pas été exportés : {error}"),
];
