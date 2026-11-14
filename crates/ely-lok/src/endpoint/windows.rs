//! Windows: two one-way pipes only this user opens.

use std::{
    fs::File,
    io::{self, Write},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::Path,
    ptr,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_LOCK_VIOLATION, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_READ_ATTRIBUTES, OPEN_EXISTING, PIPE_ACCESS_INBOUND,
        PIPE_ACCESS_OUTBOUND, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
    },
    System::{
        Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PeekNamedPipe,
            PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
        },
        Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, TerminateProcess},
    },
};

pub(crate) type Half = File;

use super::Server;

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
    let mut text = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
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
        let rule = wide(&format!("D:P(A;;GA;;;{sid})"));
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

/// The worker's pipes and server, refused unless this user's.
pub(crate) fn connect(name: &str, by: Server) -> io::Result<(Half, Half, Peer)> {
    let sid = user_sid(unsafe { GetCurrentProcess() })?;
    let (input, server) = open(&path(&sid, name, "in"), GENERIC_WRITE | FILE_READ_ATTRIBUTES, &sid, by)?;
    let (output, other) = open(&path(&sid, name, "out"), GENERIC_READ, &sid, Server::Spawned(server))?;
    if other != server {
        return Err(refused(format!("{name}'s pipes are served by {server} and {other}")));
    }
    let peer = Peer::of(server, &output)?;
    Ok((output, input, peer))
}

/// The engine behind a connection, ended by handle.
pub(crate) struct Peer {
    process: OwnedHandle,
    server: u32,
}

impl Peer {
    /// Opened while connected, so the pid is still its.
    fn of(server: u32, pipe: &File) -> io::Result<Self> {
        let handle = unsafe { OpenProcess(PROCESS_TERMINATE, 0, server) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let process = unsafe { OwnedHandle::from_raw_handle(handle) };
        // A live pipe means the handle is the server's.
        let peeked = unsafe { PeekNamedPipe(pipe.as_raw_handle() as HANDLE, ptr::null_mut(), 0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
        if peeked == 0 {
            return Err(io::Error::new(io::ErrorKind::ConnectionReset, format!("engine {server} left: {}", io::Error::last_os_error())));
        }
        Ok(Self { process, server })
    }

    /// Windows names the pipe's server.
    pub(crate) fn server(&self) -> Option<u32> {
        Some(self.server)
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

fn open(path: &str, access: u32, sid: &str, by: Server) -> io::Result<(File, u32)> {
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
    let mut server = 0u32;
    if unsafe { GetNamedPipeServerProcessId(handle, &mut server) } == 0 {
        return Err(named(io::Error::last_os_error(), path));
    }
    if matches!(by, Server::Spawned(pid) if pid != server) {
        return Err(refused(format!("{path} is served by process {server}, not the spawned worker")));
    }
    let owner = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, server) };
    if owner.is_null() {
        return Err(named(io::Error::last_os_error(), path));
    }
    let user = user_sid(owner);
    unsafe { CloseHandle(owner) };
    if user? != sid {
        return Err(refused(format!("{path} is served by process {server} of another user")));
    }
    Ok((pipe, server))
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, why)
}
