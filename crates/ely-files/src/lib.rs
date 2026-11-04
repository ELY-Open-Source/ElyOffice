//! File sessions, saving and recovery (PRD §6).

mod crash;
mod key;
mod names;
mod policy;
pub mod recent;
mod recovery;
mod rename;
mod save;
mod session;
mod task;
mod turns;

pub use crash::{Crash, Marker, alive};
pub use key::{Backend, RecoveryKey};
pub use names::same_name;
pub use policy::{Action, DEBOUNCE, Policy, PolicyEngine};
pub use recovery::{Recovery, RecoveryPoint, TooBig, restore_copy};
pub use rename::rename_new;
pub use task::RecoveryTask;
pub use turns::Turns;

pub use save::{
    Done, SaveError, Saved, Write, atomic_capable, claim_all, export, import_attachment, same_file,
    save_atomic, save_with_backup, write_attachment,
};
#[cfg(test)]
pub(crate) use save::save;
pub use session::{Fingerprint, Key, Open, SaveState, SaveTicket, Session, Sessions, StaleTicket, Stamp};
