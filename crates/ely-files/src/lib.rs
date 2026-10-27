//! File sessions, saving and recovery (PRD §6).

mod crash;
mod key;
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
pub use policy::{Action, DEBOUNCE, Policy, PolicyEngine};
pub use recovery::{Recovery, RecoveryPoint, restore_copy};
pub use rename::rename_new;
pub use task::RecoveryTask;
pub use turns::Turns;

pub use save::{
    SaveError, Saved, atomic_capable, claim_all, export, import_attachment, same_file, save,
    write_attachment,
    save_as, save_atomic, save_overwrite, save_with_backup, save_with_probe,
};
pub use session::{Fingerprint, Open, SaveState, SaveTicket, Session, Sessions, StaleTicket};
