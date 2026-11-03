//! French drafts messages.

pub(super) const FR: &[(&str, &str)] = &[
    ("mail.boxes.drafts", "Brouillons"),
    ("mail.drafts.pick", "Choisissez un brouillon pour le voir"),
    ("mail.drafts.to", "À"),
    ("mail.drafts.cc", "Cc"),
    ("mail.drafts.bcc", "Cci"),
    ("mail.drafts.saved", "Enregistré"),
    ("mail.drafts.continue", "Continuer à écrire"),
    ("mail.drafts.delete", "Supprimer le brouillon…"),
    ("mail.drafts.kept", "Le message est gardé dans les brouillons"),
    ("mail.drafts.unkept", "Le brouillon n’a pas été enregistré : {error}"),
    ("mail.drafts.undropped", "Le brouillon n’a pas été supprimé : {error}"),
    ("mail.drafts.unread", "Les brouillons n’ont pas pu être lus : {error}"),
    ("mail.drafts.up", "Garder aussi les brouillons dans le dossier Brouillons du serveur"),
    ("mail.drafts.unsaved_choice", "Le choix des brouillons n’a pas été enregistré : {error}"),
    ("mail.drafts.no_folder", "Gardé sur cet ordinateur. Le serveur ne liste pas encore de dossier Brouillons."),
    ("mail.drafts.server_failed", "Gardé sur cet ordinateur. La copie du serveur n’a pas été enregistrée : {error}"),
    ("mail.drafts.server_stale", "Une ancienne copie peut rester dans les brouillons du serveur : {error}"),
];
