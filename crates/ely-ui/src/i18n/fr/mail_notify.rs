//! French mail notification and file messages.

pub(super) const FR: &[(&str, &str)] = &[
    ("mail.notify.title", "Nouveau courrier"),
    ("mail.notify.body.one", "De {senders}"),
    ("mail.notify.body", "{count} nouveaux messages de {senders}"),
    ("mail.notify.more", "{senders} et d’autres"),
    ("mail.file.downloading", "Téléchargement du message avec ses fichiers…"),
    ("mail.file.download_failed", "Les fichiers ne se sont pas téléchargés : {error}"),
    ("mail.file.unread", "Le fichier n’a pas pu être lu : {error}"),
    ("mail.file.unsaved", "Le fichier n’a pas été enregistré : {error}"),
    ("mail.file.saved", "{name} enregistré"),
    ("mail.file.unopened", "Le fichier ne s’est pas ouvert : {error}"),
    ("mail.file.unsafe", "Ce type de fichier peut lancer un programme ; il n’est donc pas ouvert ici. Enregistrez-le, et ouvrez-le vous-même si vous faites confiance à son expéditeur."),
];
