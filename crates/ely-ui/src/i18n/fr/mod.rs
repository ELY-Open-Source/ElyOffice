//! French for every app.

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

pub(super) const FR: [&[(&str, &str)]; 34] = [
    base::FR,
    office::FR,
    doc::FR,
    refs::FR,
    mail::FR,
    mail_write::FR,
    mail_drafts::FR,
    mail_outbox::FR,
    mail_organize::FR,
    mail_contacts::FR,
    mail_migrate::FR,
    mail_notify::FR,
    mail_storage::FR,
    mail_rules::FR,
    mail_search::FR,
    mail_stages::FR,
    tools::FR,
    table::FR,
    calc::FR,
    data::FR,
    analysis::FR,
    output::FR,
    show::FR,
    motion::FR,
    play::FR,
    pdf::FR,
    pdf_steps::FR,
    secure::FR,
    tree::FR,
    tree_ops::FR,
    tree_places::FR,
    mark::FR,
    mark_menus::FR,
    settings::FR,
];
