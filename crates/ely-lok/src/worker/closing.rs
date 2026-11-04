//! Kernel dialogs, closed once the kernel's callback flush returns.

use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::ffi::Raw;

#[cfg(windows)]
use super::messages::nudge;
#[cfg(target_os = "linux")]
use super::unipoll::nudge;

/// Dialogs to close: document id, handle, window.
static DUE: Mutex<Vec<(u64, Raw, u32)>> = Mutex::new(Vec::new());

/// The queue; a panic cannot leave it half changed.
fn due() -> MutexGuard<'static, Vec<(u64, Raw, u32)>> {
    DUE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Called inside a kernel callback, where closing re-enters its flush.
pub(crate) fn later(id: u64, raw: Raw, window: u32) {
    due().push((id, raw, window));
    nudge();
}

/// Closes due windows; requests may hold the docs lock.
pub(super) fn now() {
    let ready = std::mem::take(&mut *due());
    for (_, raw, window) in ready {
        crate::ffi::close_window(raw, window);
    }
}

/// Drops the dialogs of a document about to be freed.
pub(super) fn forget(id: u64) {
    due().retain(|(due, ..)| {
        let keep = *due != id;
        if !keep {
            log::info!("lok-worker: a dialog of document {id} closed with it");
        }
        keep
    });
}

#[cfg(target_os = "macos")]
use run_loop::nudge;
#[cfg(target_os = "macos")]
pub(super) use run_loop::watch;

/// macOS: a run loop source, which modal loops pump too.
#[cfg(target_os = "macos")]
mod run_loop {
    use std::{
        ffi::c_void,
        io,
        ptr::{null, null_mut},
        sync::atomic::{AtomicPtr, Ordering},
    };

    /// `CFRunLoopSourceContext`, version 0.
    #[repr(C)]
    struct Context {
        version: isize,
        info: *mut c_void,
        retain: *const c_void,
        release: *const c_void,
        describe: *const c_void,
        equal: *const c_void,
        hash: *const c_void,
        schedule: *const c_void,
        cancel: *const c_void,
        perform: extern "C" fn(*mut c_void),
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFRunLoopCommonModes: *const c_void;
        fn CFRunLoopGetMain() -> *mut c_void;
        fn CFRunLoopSourceCreate(allocator: *const c_void, order: isize, context: *mut Context) -> *mut c_void;
        fn CFRunLoopAddSource(run_loop: *mut c_void, source: *mut c_void, mode: *const c_void);
        fn CFRunLoopSourceSignal(source: *mut c_void);
        fn CFRunLoopWakeUp(run_loop: *mut c_void);
    }

    static SOURCE: AtomicPtr<c_void> = AtomicPtr::new(null_mut());

    extern "C" fn due(_: *mut c_void) {
        super::now();
    }

    /// Adds the source to the main run loop.
    pub(in crate::worker) fn watch() -> io::Result<()> {
        let mut context = Context {
            version: 0,
            info: null_mut(),
            retain: null(),
            release: null(),
            describe: null(),
            equal: null(),
            hash: null(),
            schedule: null(),
            cancel: null(),
            perform: due,
        };
        let source = unsafe { CFRunLoopSourceCreate(null(), 0, &mut context) };
        if source.is_null() {
            return Err(io::Error::other("the dialog source was not created"));
        }
        unsafe { CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes) };
        SOURCE.store(source, Ordering::SeqCst);
        Ok(())
    }

    /// Unlike GCD's main queue, modal loops pump this.
    pub(super) fn nudge() {
        let source = SOURCE.load(Ordering::SeqCst);
        if source.is_null() {
            log::error!("lok-worker: a dialog close before the loop");
            return;
        }
        unsafe {
            CFRunLoopSourceSignal(source);
            CFRunLoopWakeUp(CFRunLoopGetMain());
        }
    }
}
