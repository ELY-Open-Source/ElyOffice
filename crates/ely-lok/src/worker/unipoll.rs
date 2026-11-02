//! Linux: LOK runs its loop; requests arrive in its poll.

use std::{
    ffi::c_void,
    io,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
        mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel},
    },
    time::Duration,
};

use super::{answer, asked, closing, events, lok, pin, quit, teardown};
use crate::ffi::Lok;
use crate::ipc;

/// A request, the kernel's wake, or the end.
enum Note {
    Ask(u8, u64, Vec<u8>),
    Woken,
    Due,
    Closed,
}

type Notes = (Mutex<Sender<Note>>, Mutex<Receiver<Note>>);

static NOTES: OnceLock<Notes> = OnceLock::new();

/// Requests read and not yet answered.
static PENDING: AtomicUsize = AtomicUsize::new(0);

pub(super) fn serve(lok: Lok, input: impl io::Read + Send + 'static) -> io::Result<()> {
    let lok = Box::leak(Box::new(lok));
    pin(lok);
    lok.register_callback(asked::on_kit);
    lok.register_any_input(waiting);
    let (send, receive) = channel();
    let reading = send.clone();
    NOTES
        .set((Mutex::new(send), Mutex::new(receive)))
        .map_err(|_| io::Error::other("the loop is served twice"))?;
    std::thread::spawn(move || reader(input, reading));
    std::thread::spawn(events::push);
    lok.run_loop(Some(poll), Some(wake));
    log::info!("lok-worker: the kernel's loop ended");
    teardown::teardown(0);
}

fn reader(mut input: impl io::Read, notes: Sender<Note>) {
    loop {
        let note = match ipc::read_frame(&mut input) {
            Ok(Some((tag, id, payload))) => {
                PENDING.fetch_add(1, Ordering::SeqCst);
                Note::Ask(tag, id, payload)
            }
            Ok(None) => {
                log::info!("lok-worker: stdin closed");
                Note::Closed
            }
            Err(error) => {
                log::warn!("lok-worker: frame read failed: {error}");
                quit(1);
            }
        };
        let closed = matches!(note, Note::Closed);
        if notes.send(note).is_err() || closed {
            return;
        }
    }
}

/// Idle jobs yield to a waiting request.
extern "C" fn waiting(_: *mut c_void, _: i32) -> bool {
    PENDING.load(Ordering::SeqCst) > 0
}

/// The kernel's wait: one request answered, or its own wake.
extern "C" fn poll(_: *mut c_void, timeout_us: i32) -> i32 {
    let notes = &NOTES.get().expect("the loop is served").1;
    let notes = notes.lock().expect("the notes lock");
    let note = match timeout_us {
        0 => notes.try_recv().map_err(|error| error == TryRecvError::Disconnected),
        wait if wait < 0 => notes.recv().map_err(|_| true),
        wait => notes
            .recv_timeout(Duration::from_micros(wait as u64))
            .map_err(|error| error == RecvTimeoutError::Disconnected),
    };
    drop(notes);
    match note {
        Ok(Note::Ask(tag, id, payload)) => {
            PENDING.fetch_sub(1, Ordering::SeqCst);
            answer(lok(), tag, id, &payload);
            1
        }
        Ok(Note::Woken) => 1,
        Ok(Note::Due) => {
            closing::now();
            1
        }
        Ok(Note::Closed) | Err(true) => teardown::teardown(0),
        Err(false) => 0,
    }
}

/// Asks the loop to close the due dialogs.
pub(super) fn nudge() {
    send(Note::Due, "a dialog close");
}

/// Another thread posted work for the kernel's loop.
extern "C" fn wake(_: *mut c_void) {
    send(Note::Woken, "a wake");
}

/// Hands the loop a note from outside its poll.
fn send(note: Note, what: &str) {
    let notes = &NOTES.get().expect("the loop is served").0;
    if notes.lock().expect("the notes lock").send(note).is_err() {
        log::info!("lok-worker: {what} after the loop ended");
    }
}
