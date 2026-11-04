//! Vietnamese for every app.

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

pub(super) const VI: [&[(&str, &str)]; 34] = [
    base::VI,
    office::VI,
    doc::VI,
    refs::VI,
    mail::VI,
    mail_write::VI,
    mail_drafts::VI,
    mail_outbox::VI,
    mail_organize::VI,
    mail_contacts::VI,
    mail_migrate::VI,
    mail_notify::VI,
    mail_storage::VI,
    mail_rules::VI,
    mail_search::VI,
    mail_stages::VI,
    tools::VI,
    table::VI,
    calc::VI,
    data::VI,
    analysis::VI,
    output::VI,
    show::VI,
    motion::VI,
    play::VI,
    pdf::VI,
    pdf_steps::VI,
    secure::VI,
    tree::VI,
    tree_ops::VI,
    tree_places::VI,
    mark::VI,
    mark_menus::VI,
    settings::VI,
];
