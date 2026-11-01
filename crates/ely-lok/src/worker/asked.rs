//! The kernel's questions during a load: password, damage.

use std::{
    ffi::{CStr, c_char, c_void},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// LOK_CALLBACK_DOCUMENT_PASSWORD.
const PASSWORD: i32 = 20;
/// LOK_CALLBACK_ERROR.
const ERROR: i32 = 22;

/// The password for the load in progress, given once.
static GIVEN: Mutex<Option<String>> = Mutex::new(None);
/// The kernel asked and got none.
static REFUSED: AtomicBool = AtomicBool::new(false);
/// The kernel found the package broken.
static DAMAGED: AtomicBool = AtomicBool::new(false);

/// Readies `password` for the next load.
pub(super) fn arm(password: Option<String>) {
    *GIVEN.lock().expect("the password lock") = password;
    REFUSED.store(false, Ordering::SeqCst);
    DAMAGED.store(false, Ordering::SeqCst);
}

/// Whether the last load lacked a right password.
pub(super) fn refused() -> bool {
    REFUSED.load(Ordering::SeqCst)
}

/// Whether the last load met a broken package.
pub(super) fn damaged() -> bool {
    DAMAGED.load(Ordering::SeqCst)
}

/// Answers once; a second ask means the password failed.
pub(super) unsafe extern "C" fn on_kit(kind: i32, payload: *const c_char, _: *mut c_void) {
    if payload.is_null() {
        return;
    }
    let said = unsafe { CStr::from_ptr(payload) }.to_string_lossy();
    if kind == ERROR {
        return noted(&said);
    }
    if kind != PASSWORD {
        return;
    }
    let url = said.into_owned();
    let given = GIVEN.lock().expect("the password lock").take();
    log::info!(
        "lok-worker: a password asked, one given: {}",
        given.is_some()
    );
    REFUSED.store(given.is_none(), Ordering::SeqCst);
    super::lok().set_password(&url, given.as_deref());
}

/// An error the kernel reports; a broken package is kept.
fn noted(said: &str) {
    let kind = serde_json::from_str::<serde_json::Value>(said).ok().and_then(|value| value["kind"].as_str().map(str::to_owned));
    log::info!("lok-worker: the kernel reported a {} error", kind.as_deref().unwrap_or("unreadable"));
    if kind.as_deref() == Some("brokenpackage") {
        DAMAGED.store(true, Ordering::SeqCst);
    }
}
