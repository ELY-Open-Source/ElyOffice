//! The name documents sign new comments and changes with.

use std::io;

/// `author` as the settings give it, else the account's name.
pub fn named(author: &str) -> io::Result<String> {
    if !author.is_empty() {
        return Ok(author.to_string());
    }
    account()
}

/// The account's full name, else its login name.
pub fn account() -> io::Result<String> {
    let full = full_name()?;
    if !full.trim().is_empty() {
        return Ok(full.trim().to_string());
    }
    log::info!("user: no full name; the login name signs");
    login()
}

/// The login name, from the system, never the environment.
#[cfg(windows)]
pub fn login() -> io::Result<String> {
    use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;
    let mut size = 0u32;
    unsafe { GetUserNameW(std::ptr::null_mut(), &mut size) };
    if size == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut buffer = vec![0u16; size as usize];
    if unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut size) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // The count includes the closing null.
    let name = String::from_utf16_lossy(&buffer[..size.saturating_sub(1) as usize]);
    nonempty(name)
}

/// The login name, from the system, never the environment.
#[cfg(unix)]
pub fn login() -> io::Result<String> {
    nonempty(passwd(|entry| text(entry.pw_name))?)
}

fn nonempty(name: String) -> io::Result<String> {
    if name.is_empty() {
        return Err(io::Error::other("the system gives this account no login name"));
    }
    Ok(name)
}

/// Reads this account's passwd entry.
#[cfg(unix)]
fn passwd<T>(read: impl FnOnce(&libc::passwd) -> T) -> io::Result<T> {
    const MOST: usize = 1 << 20;
    let mut entry: libc::passwd = unsafe { std::mem::zeroed() };
    let mut found = std::ptr::null_mut();
    let mut buffer = vec![0 as libc::c_char; 1024];
    loop {
        let code = unsafe {
            libc::getpwuid_r(libc::getuid(), &mut entry, buffer.as_mut_ptr(), buffer.len(), &mut found)
        };
        if code == libc::ERANGE && buffer.len() < MOST {
            buffer.resize(buffer.len() * 2, 0);
            continue;
        }
        if code != 0 {
            return Err(io::Error::from_raw_os_error(code));
        }
        if found.is_null() {
            return Err(io::Error::other("this account has no passwd entry"));
        }
        return Ok(read(&entry));
    }
}

#[cfg(unix)]
fn text(field: *const libc::c_char) -> String {
    if field.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(field) }.to_string_lossy().into_owned()
}

#[cfg(target_os = "macos")]
fn full_name() -> io::Result<String> {
    Ok(objc2_foundation::NSFullUserName().to_string())
}

#[cfg(target_os = "linux")]
fn full_name() -> io::Result<String> {
    // The first comma field of the passwd comment.
    passwd(|entry| text(entry.pw_gecos).split(',').next().unwrap_or_default().to_string())
}

#[cfg(windows)]
fn full_name() -> io::Result<String> {
    use windows_sys::Win32::Security::Authentication::Identity::{GetUserNameExW, NameDisplay};
    let mut size = 0u32;
    unsafe { GetUserNameExW(NameDisplay, std::ptr::null_mut(), &mut size) };
    // Accounts outside a domain have no display name.
    if size == 0 {
        return Ok(String::new());
    }
    let mut buffer = vec![0u16; size as usize];
    if !unsafe { GetUserNameExW(NameDisplay, buffer.as_mut_ptr(), &mut size) } {
        return Err(io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&buffer[..size as usize]))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_set_name_signs_else_the_account() {
        assert_eq!(super::named("Ada").expect("a name"), "Ada");
        let account = super::named("").expect("a name");
        assert!(!account.is_empty());
        assert_eq!(account, super::account().expect("the account"));
    }

    const CHILD: &str = "ELY_USER_TEST_CHILD";
    const NAMES: [&str; 3] = ["USER", "USERNAME", "LOGNAME"];

    #[test]
    fn the_account_names_itself_without_its_environment() {
        if std::env::var_os(CHILD).is_some() {
            assert!(NAMES.iter().all(|key| std::env::var_os(key).is_none()));
            assert!(!super::account().expect("an account with no name variables").is_empty());
            assert!(!super::login().expect("a login with no name variables").is_empty());
            return;
        }
        let login = super::login().expect("the login name");
        let given = NAMES.iter().find_map(|key| std::env::var(key).ok());
        if let Some(given) = given {
            assert!(login.eq_ignore_ascii_case(&given), "{login} is not {given}");
        }
        let mut child = std::process::Command::new(std::env::current_exe().expect("the test binary"));
        child.args(["--exact", "user::tests::the_account_names_itself_without_its_environment"]);
        for key in NAMES {
            child.env_remove(key);
        }
        let status = child.env(CHILD, "1").status().expect("the child test runs");
        assert!(status.success(), "the child failed: {status}");
    }
}
