//! A job object holds R from its first instruction.

use std::{
    io,
    os::windows::{
        io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle},
        process::CommandExt as _,
    },
    process::{Child, Command},
};

use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        },
        JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject,
        },
        Threading::{
            CREATE_NO_WINDOW, CREATE_SUSPENDED, OpenThread, ResumeThread, THREAD_SUSPEND_RESUME,
        },
    },
};

/// Closing it ends every process inside.
pub struct Job(OwnedHandle);

fn owned(handle: HANDLE) -> io::Result<OwnedHandle> {
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh handle nothing else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

fn checked(done: i32) -> io::Result<()> {
    match done {
        0 => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

/// Starts suspended, joins the job, then runs.
pub fn start(command: &mut Command) -> io::Result<(Child, Job)> {
    // SAFETY: no name, default security.
    let job = Job(owned(unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) })?);
    // SAFETY: zero is a valid limit record.
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: the record outlives the call.
    checked(unsafe {
        SetInformationJobObject(
            job.0.as_raw_handle(),
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    })?;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    let mut child = command.spawn()?;
    match admit(&job, &child) {
        Ok(()) => Ok((child, job)),
        Err(error) => {
            child.kill()?;
            child.wait()?;
            Err(error)
        }
    }
}

fn admit(job: &Job, child: &Child) -> io::Result<()> {
    // SAFETY: both handles are open.
    checked(unsafe { AssignProcessToJobObject(job.0.as_raw_handle(), child.as_raw_handle()) })?;
    let thread = owned(unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, first_thread(child.id())?) })?;
    // SAFETY: the thread handle is open.
    match unsafe { ResumeThread(thread.as_raw_handle()) } {
        u32::MAX => Err(io::Error::last_os_error()),
        _ => Ok(()),
    }
}

/// A suspended new process has one thread.
fn first_thread(pid: u32) -> io::Result<u32> {
    // SAFETY: a snapshot of all threads.
    let snapshot = owned(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) })?;
    // SAFETY: zero, then the size the API asks.
    let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
    entry.dwSize = size_of::<THREADENTRY32>() as u32;
    // SAFETY: the entry outlives each call.
    let mut more = unsafe { Thread32First(snapshot.as_raw_handle(), &mut entry) };
    while more != 0 {
        if entry.th32OwnerProcessID == pid {
            return Ok(entry.th32ThreadID);
        }
        more = unsafe { Thread32Next(snapshot.as_raw_handle(), &mut entry) };
    }
    Err(io::Error::other(format!("no thread of process {pid}")))
}

impl Job {
    pub fn end(&self) -> io::Result<()> {
        // SAFETY: the job handle is open.
        checked(unsafe { TerminateJobObject(self.0.as_raw_handle(), 1) })
    }
}

#[cfg(test)]
pub use probe::{alive, child_of};

#[cfg(test)]
mod probe {
    use super::{checked, owned};
    use std::{
        io,
        os::windows::io::AsRawHandle as _,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::WAIT_TIMEOUT,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        },
    };

    pub fn alive(pid: u32) -> bool {
        let Ok(process) = owned(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) }) else {
            return false;
        };
        unsafe { WaitForSingleObject(process.as_raw_handle(), 0) == WAIT_TIMEOUT }
    }

    /// The process `name` that `parent` starts.
    pub fn child_of(parent: u32, name: &str, wait: Duration) -> io::Result<u32> {
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline {
            let snapshot = owned(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) })?;
            let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
            checked(unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) })?;
            loop {
                let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                if entry.th32ParentProcessID == parent && exe.eq_ignore_ascii_case(name) {
                    return Ok(entry.th32ProcessID);
                }
                if unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) } == 0 {
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err(io::Error::other(format!("{parent} started no {name}")))
    }
}
