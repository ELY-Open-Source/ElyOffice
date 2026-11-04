//! German for every app.

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

pub(super) const DE: [&[(&str, &str)]; 34] = [
    base::DE,
    office::DE,
    doc::DE,
    refs::DE,
    mail::DE,
    mail_write::DE,
    mail_drafts::DE,
    mail_outbox::DE,
    mail_organize::DE,
    mail_contacts::DE,
    mail_migrate::DE,
    mail_notify::DE,
    mail_storage::DE,
    mail_rules::DE,
    mail_search::DE,
    mail_stages::DE,
    tools::DE,
    table::DE,
    calc::DE,
    data::DE,
    analysis::DE,
    output::DE,
    show::DE,
    motion::DE,
    play::DE,
    pdf::DE,
    pdf_steps::DE,
    secure::DE,
    tree::DE,
    tree_ops::DE,
    tree_places::DE,
    mark::DE,
    mark_menus::DE,
    settings::DE,
];
