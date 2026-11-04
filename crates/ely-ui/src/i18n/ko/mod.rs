//! Korean for every app.

mod analysis;
mod base;
mod calc;
mod data;
mod doc;
mod mail;
mod mail_contacts;
mod mail_drafts;
mod mail_migrate;
mod mail_notify;
mod mail_organize;
mod mail_outbox;
mod mail_rules;
mod mail_search;
mod mail_stages;
mod mail_storage;
mod mail_write;
mod mark;
mod mark_menus;
mod motion;
mod office;
mod output;
mod pdf;
mod pdf_steps;
mod play;
mod refs;
mod secure;
mod settings;
mod show;
mod table;
mod tools;
mod tree;
mod tree_ops;
mod tree_places;

pub(super) const KO: [&[(&str, &str)]; 34] = [
    base::KO,
    office::KO,
    doc::KO,
    refs::KO,
    mail::KO,
    mail_write::KO,
    mail_drafts::KO,
    mail_outbox::KO,
    mail_organize::KO,
    mail_contacts::KO,
    mail_migrate::KO,
    mail_notify::KO,
    mail_storage::KO,
    mail_rules::KO,
    mail_search::KO,
    mail_stages::KO,
    tools::KO,
    table::KO,
    calc::KO,
    data::KO,
    analysis::KO,
    output::KO,
    show::KO,
    motion::KO,
    play::KO,
    pdf::KO,
    pdf_steps::KO,
    secure::KO,
    tree::KO,
    tree_ops::KO,
    tree_places::KO,
    mark::KO,
    mark_menus::KO,
    settings::KO,
];
