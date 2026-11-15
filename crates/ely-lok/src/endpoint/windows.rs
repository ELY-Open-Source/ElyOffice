//! Windows: two one-way pipes only this user opens.

use std::{
    fs::File,
    io::{self, Write},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::Path,
    ptr,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, ERROR_LOCK_VIOLATION, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1, SE_KERNEL_OBJECT},
        GetTokenInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_READ_ATTRIBUTES, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
        PIPE_ACCESS_OUTBOUND, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
    },
    System::{
        Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, PeekNamedPipe,
            PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
        },
        Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess},
    },
};

pub(crate) type Half = File;

unsafe extern "C" {
    fn _dup2(from: i32, to: i32) -> i32;
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn named(error: io::Error, path: &str) -> io::Error {
    io::Error::new(error.kind(), format!("{path}: {error}"))
}

/// A process's user, as an SID string.
fn user_sid(process: HANDLE) -> io::Result<String> {
    let mut token: HANDLE = ptr::null_mut();
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut size = 0u32;
    unsafe { GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut size) };
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    let read = unsafe { GetTokenInformation(token, TokenUser, buffer.as_mut_ptr().cast(), size, &mut size) };
    let failed = io::Error::last_os_error();
    unsafe { CloseHandle(token) };
    if read == 0 {
        return Err(failed);
    }
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    sid_text(user.User.Sid)
}

fn sid_text(sid: PSID) -> io::Result<String> {
    let mut text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let len = (0..).take_while(|&at| unsafe { *text.add(at) } != 0).count();
    let sid = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) });
    unsafe { LocalFree(text.cast()) };
    Ok(sid)
}

/// Pipe `way`: "in" or "out".
fn path(sid: &str, name: &str, way: &str) -> String {
    format!(r"\\.\pipe\ely-office-{sid}-{name}-{way}")
}

/// Both pipes, open to this user alone.
pub(crate) struct Listener {
    input: File,
    output: File,
}

impl Listener {
    /// Private names come first; idle ones admit successors.
    pub(crate) fn bind(name: &str, idle: bool) -> io::Result<Self> {
        let sid = user_sid(unsafe { GetCurrentProcess() })?;
        let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
        // The owner is named: clients check it, not a pid.
        let rule = wide(&format!("O:{sid}D:P(A;;GA;;;{sid})"));
        let made = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(rule.as_ptr(), SDDL_REVISION_1, &mut descriptor, ptr::null_mut())
        };
        if made == 0 {
            return Err(io::Error::last_os_error());
        }
        let only_me = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let pipes = create(&path(&sid, name, "in"), PIPE_ACCESS_INBOUND, &only_me, idle)
            .and_then(|input| Ok((input, create(&path(&sid, name, "out"), PIPE_ACCESS_OUTBOUND, &only_me, idle)?)));
        unsafe { LocalFree(descriptor) };
        let (input, output) = pipes?;
        log::info!("lok-worker: listening on {}", path(&sid, name, "*"));
        Ok(Self { input, output })
    }

    /// The next app in line.
    pub(crate) fn accept(&self) -> io::Result<(Half, Half)> {
        wait(&self.input)?;
        wait(&self.output)?;
        Ok((self.input.try_clone()?, self.output.try_clone()?))
    }

    /// The app's handles keep the pipes; these go.
    pub(crate) fn close(self) -> io::Result<()> {
        Ok(())
    }
}

fn create(path: &str, access: u32, only_me: &SECURITY_ATTRIBUTES, idle: bool) -> io::Result<File> {
    let mode = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    let (flags, instances) = match idle {
        true => (access, PIPE_UNLIMITED_INSTANCES),
        false => (access | FILE_FLAG_FIRST_PIPE_INSTANCE, 1),
    };
    let handle = unsafe { CreateNamedPipeW(wide(path).as_ptr(), flags, mode, instances, 1 << 16, 1 << 16, 0, only_me) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(named(io::Error::last_os_error(), path));
    }
    Ok(unsafe { File::from_raw_handle(handle) })
}

fn wait(pipe: &File) -> io::Result<()> {
    if unsafe { ConnectNamedPipe(pipe.as_raw_handle() as HANDLE, ptr::null_mut()) } != 0 {
        return Ok(());
    }
    match unsafe { GetLastError() } {
        ERROR_PIPE_CONNECTED => Ok(()),
        code => Err(io::Error::from_raw_os_error(code as i32)),
    }
}

/// Says it listens, then stdout joins stderr.
pub(crate) fn announce(line: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{line}")?;
    out.flush()?;
    if unsafe { _dup2(2, 1) } != 0 {
        return Err(io::Error::other("stdout did not join stderr"));
    }
    if unsafe { SetStdHandle(STD_OUTPUT_HANDLE, GetStdHandle(STD_ERROR_HANDLE)) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// The worker's pipes, refused unless this user owns them.
pub(crate) fn connect(name: &str) -> io::Result<(Half, Half)> {
    let sid = user_sid(unsafe { GetCurrentProcess() })?;
    let input = open(&path(&sid, name, "in"), GENERIC_WRITE | FILE_READ_ATTRIBUTES, &sid)?;
    let output = open(&path(&sid, name, "out"), GENERIC_READ, &sid)?;
    Ok((output, input))
}

/// The engine behind a connection, ended by handle.
pub(crate) struct Peer {
    process: OwnedHandle,
}

impl Peer {
    /// The pid it gave; held while connected, so never reused.
    pub(crate) fn of(pid: u32, pipe: &File) -> io::Result<Self> {
        // Windows names the pipe's first creator, not the speaker.
        let handle = unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            let error = io::Error::last_os_error();
            // An exited pid opens as an invalid parameter.
            let kind = match error.raw_os_error() == Some(ERROR_INVALID_PARAMETER as i32) {
                true => io::ErrorKind::ConnectionReset,
                false => error.kind(),
            };
            return Err(io::Error::new(kind, format!("engine {pid}: {error}")));
        }
        let process = unsafe { OwnedHandle::from_raw_handle(handle) };
        let (user, ours) = (user_sid(handle)?, user_sid(unsafe { GetCurrentProcess() })?);
        if user != ours {
            return Err(refused(format!("engine {pid} runs as another user")));
        }
        let peeked = unsafe { PeekNamedPipe(pipe.as_raw_handle() as HANDLE, ptr::null_mut(), 0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
        if peeked == 0 {
            return Err(io::Error::new(io::ErrorKind::ConnectionReset, format!("engine {pid} left: {}", io::Error::last_os_error())));
        }
        Ok(Self { process })
    }

    /// Ends the engine; its pid is never reused while held.
    pub(crate) fn end(&self) -> io::Result<()> {
        match unsafe { TerminateProcess(self.process.as_raw_handle() as HANDLE, 1) } {
            0 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        }
    }
}

/// Locks `path` for this process, without truncating it.
pub(crate) fn hold(path: &Path) -> io::Result<File> {
    let file = File::options().read(true).write(true).create(true).truncate(false).open(path).map_err(|error| at(error, path))?;
    // An empty file has no range to probe.
    if file.metadata().map_err(|error| at(error, path))?.len() == 0 {
        match (&file).write_all(&[0]) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) => return Err(busy()),
            Err(error) => return Err(at(error, path)),
        }
    }
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(busy()),
        Err(std::fs::TryLockError::Error(error)) => Err(at(error, path)),
    }
}

/// Whether some process holds `path`; takes nothing.
pub(crate) fn held(path: &Path) -> io::Result<bool> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(at(error, path)),
    };
    match io::Read::read(&mut file, &mut [0]) {
        Ok(_) => Ok(false),
        Err(error) if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) => Ok(true),
        Err(error) => Err(at(error, path)),
    }
}

fn busy() -> io::Error {
    io::Error::new(io::ErrorKind::AlreadyExists, "another idle engine waits for this app")
}

fn at(error: io::Error, path: &Path) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// Ends our side; the engine sees its pipes close.
pub(crate) fn hang_up(half: Half) -> io::Result<()> {
    drop(half);
    Ok(())
}

fn open(path: &str, access: u32, sid: &str) -> io::Result<File> {
    // The server may identify us, never act as us.
    let flags = SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION;
    let handle = unsafe { CreateFileW(wide(path).as_ptr(), access, 0, ptr::null(), OPEN_EXISTING, flags, ptr::null_mut()) };
    if handle == INVALID_HANDLE_VALUE {
        let error = io::Error::last_os_error();
        // Another app holds it; the caller asks the lock.
        if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) {
            return Err(io::Error::new(io::ErrorKind::ResourceBusy, format!("{path} is taken: {error}")));
        }
        return Err(named(error, path));
    }
    let pipe = unsafe { File::from_raw_handle(handle) };
    let owner = pipe_owner(handle).map_err(|error| named(error, path))?;
    if owner != sid {
        return Err(refused(format!("{path} is owned by {owner}, another user")));
    }
    Ok(pipe)
}

/// The SID that owns the pipe.
fn pipe_owner(handle: HANDLE) -> io::Result<String> {
    let (mut owner, mut descriptor): (PSID, PSECURITY_DESCRIPTOR) = (ptr::null_mut(), ptr::null_mut());
    let code = unsafe {
        GetSecurityInfo(handle, SE_KERNEL_OBJECT, OWNER_SECURITY_INFORMATION, &mut owner, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut descriptor)
    };
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code as i32));
    }
    let text = sid_text(owner);
    unsafe { LocalFree(descriptor) };
    text
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, why)
}
