//! Spanish for Doc, Table and their shared windows.

mod analysis;
mod base;
mod calc;
mod data;
mod doc;
mod office;
mod output;
mod refs;
mod settings;
mod table;
mod tools;

use super::Catalog;

pub(super) fn catalog() -> Catalog {
    base::ES
        .iter()
        .chain(office::ES)
        .chain(doc::ES)
        .chain(refs::ES)
        .chain(tools::ES)
        .chain(table::ES)
        .chain(calc::ES)
        .chain(data::ES)
        .chain(analysis::ES)
        .chain(output::ES)
        .chain(settings::ES)
        .copied()
        .collect()
}
