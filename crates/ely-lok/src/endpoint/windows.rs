//! Windows: two one-way pipes only this user opens.

use std::{
    fs::File,
    io::{self, Write},
    os::windows::io::{AsRawHandle, FromRawHandle},
    ptr,
};

use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree},
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
            ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeServerProcessId, PIPE_READMODE_BYTE,
            PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT,
        },
        Threading::{GetCurrentProcess, OpenProcessToken},
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

/// This process's user, as an SID string.
fn user_sid() -> io::Result<String> {
    let mut token: HANDLE = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
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

/// Both pipes, made first, open to this user alone.
pub(crate) struct Listener {
    input: File,
    output: File,
}

impl Listener {
    pub(crate) fn bind(name: &str) -> io::Result<Self> {
        let sid = user_sid()?;
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
        let pipes = create(&path(&sid, name, "in"), PIPE_ACCESS_INBOUND, &only_me)
            .and_then(|input| Ok((input, create(&path(&sid, name, "out"), PIPE_ACCESS_OUTBOUND, &only_me)?)));
        unsafe { LocalFree(descriptor) };
        let (input, output) = pipes?;
        log::info!("lok-worker: listening on {}", path(&sid, name, "*"));
        Ok(Self { input, output })
    }

    /// The first app; one instance each, no one else connects.
    pub(crate) fn accept(self) -> io::Result<(Half, Half)> {
        wait(&self.input)?;
        wait(&self.output)?;
        Ok((self.input, self.output))
    }
}

fn create(path: &str, access: u32, only_me: &SECURITY_ATTRIBUTES) -> io::Result<File> {
    let mode = PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS;
    let flags = access | FILE_FLAG_FIRST_PIPE_INSTANCE;
    let handle = unsafe { CreateNamedPipeW(wide(path).as_ptr(), flags, mode, 1, 1 << 16, 1 << 16, 0, only_me) };
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

/// The worker's pipes, refused unless process `pid` made them.
pub(crate) fn connect(name: &str, pid: u32) -> io::Result<(Half, Half)> {
    let sid = user_sid()?;
    let input = open(&path(&sid, name, "in"), GENERIC_WRITE | FILE_READ_ATTRIBUTES, pid)?;
    let output = open(&path(&sid, name, "out"), GENERIC_READ, pid)?;
    Ok((output, input))
}

fn open(path: &str, access: u32, pid: u32) -> io::Result<File> {
    // The server may identify us, never act as us.
    let flags = SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION;
    let handle = unsafe { CreateFileW(wide(path).as_ptr(), access, 0, ptr::null(), OPEN_EXISTING, flags, ptr::null_mut()) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(named(io::Error::last_os_error(), path));
    }
    let pipe = unsafe { File::from_raw_handle(handle) };
    let mut server = 0u32;
    if unsafe { GetNamedPipeServerProcessId(handle, &mut server) } == 0 {
        return Err(named(io::Error::last_os_error(), path));
    }
    if server != pid {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{path} is served by process {server}, not worker {pid}"),
        ));
    }
    Ok(pipe)
}
