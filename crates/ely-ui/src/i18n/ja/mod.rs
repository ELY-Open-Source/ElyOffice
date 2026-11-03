//! Japanese for every app.

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

pub(super) const JA: [&[(&str, &str)]; 34] = [
    base::JA,
    office::JA,
    doc::JA,
    refs::JA,
    mail::JA,
    mail_write::JA,
    mail_drafts::JA,
    mail_outbox::JA,
    mail_organize::JA,
    mail_contacts::JA,
    mail_migrate::JA,
    mail_notify::JA,
    mail_storage::JA,
    mail_rules::JA,
    mail_search::JA,
    mail_stages::JA,
    tools::JA,
    table::JA,
    calc::JA,
    data::JA,
    analysis::JA,
    output::JA,
    show::JA,
    motion::JA,
    play::JA,
    pdf::JA,
    pdf_steps::JA,
    secure::JA,
    tree::JA,
    tree_ops::JA,
    tree_places::JA,
    mark::JA,
    mark_menus::JA,
    settings::JA,
];
