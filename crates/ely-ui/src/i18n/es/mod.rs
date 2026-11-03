//! Spanish for every app.

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

pub(super) const ES: [&[(&str, &str)]; 34] = [
    base::ES,
    office::ES,
    doc::ES,
    refs::ES,
    mail::ES,
    mail_write::ES,
    mail_drafts::ES,
    mail_outbox::ES,
    mail_organize::ES,
    mail_contacts::ES,
    mail_migrate::ES,
    mail_notify::ES,
    mail_storage::ES,
    mail_rules::ES,
    mail_search::ES,
    mail_stages::ES,
    tools::ES,
    table::ES,
    calc::ES,
    data::ES,
    analysis::ES,
    output::ES,
    show::ES,
    motion::ES,
    play::ES,
    pdf::ES,
    pdf_steps::ES,
    secure::ES,
    tree::ES,
    tree_ops::ES,
    tree_places::ES,
    mark::ES,
    mark_menus::ES,
    settings::ES,
];
