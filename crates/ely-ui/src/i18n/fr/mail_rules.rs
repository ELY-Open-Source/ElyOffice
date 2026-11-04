//! French mail rules messages.

pub(super) const FR: &[(&str, &str)] = &[
    ("mail.rules.field", "Regarder"),
    ("mail.rules.from", "De"),
    ("mail.rules.to", "À ou Cc"),
    ("mail.rules.subject", "Objet"),
    ("mail.rules.words", "Mots à chercher"),
    ("mail.rules.action", "Alors"),
    ("mail.rules.flag", "le marquer d’un drapeau"),
    ("mail.rules.archive", "l’archiver"),
    ("mail.rules.move", "le déplacer vers {name}"),
    ("mail.rules.add", "Ajouter une règle"),
    ("mail.rules.remove", "Supprimer la règle"),
    ("mail.rules.said", "Nouveau courrier dont le champ {field} contient « {words} » : {action}"),
    ("mail.rules.unsaved", "La règle n’a pas été enregistrée : {error}"),
    ("mail.rules.unread", "Les règles n’ont pas pu être lues : {error}"),
    ("mail.rules.refused", "Une règle n’a pas pu agir : {error}"),
];
