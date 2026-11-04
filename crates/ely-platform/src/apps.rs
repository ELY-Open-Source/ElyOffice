//! Starting another Ely app installed beside this one.

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

use crate::AppId;

/// Where `app` sits for an install laid out like `exe`'s.
pub(crate) fn beside(exe: &Path, app: AppId) -> Vec<PathBuf> {
    let file = format!("{}{}", app.slug(), std::env::consts::EXE_SUFFIX);
    let Some(dir) = exe.parent() else {
        return Vec::new();
    };
    let mut places = vec![dir.join(&file)];
    // macOS: Name.app/Contents/MacOS; Windows: Ely Office\Name.
    if let Some(bundle) = dir.parent().and_then(Path::parent) {
        let app_dir = format!("{}.app", app.name());
        if let Some(apps) = bundle.parent() {
            places.push(apps.join(app_dir).join("Contents/MacOS").join(&file));
        }
    }
    if let Some(suite) = dir.parent() {
        places.push(suite.join(app.name()).join(&file));
    }
    // Linux packages: /opt/ely-office/<slug>/bin.
    if let Some(opt) = dir.parent().and_then(Path::parent) {
        places.push(opt.join(app.slug()).join("bin").join(&file));
    }
    places
}

/// `app`'s binary: beside this one, else wherever macOS has it.
fn find(app: AppId) -> io::Result<Option<PathBuf>> {
    let exe = std::env::current_exe()?;
    let near = beside(&exe, app).into_iter().find(|place| place.is_file());
    Ok(near.or_else(|| registered(app)))
}

#[cfg(target_os = "macos")]
pub(crate) fn registered(app: AppId) -> Option<PathBuf> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;
    let id = NSString::from_str(&app.bundle_id());
    let url = NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&id)?;
    let bundle = PathBuf::from(url.path()?.to_string());
    let binary = bundle.join("Contents/MacOS").join(app.slug());
    binary.is_file().then_some(binary)
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn registered(_: AppId) -> Option<PathBuf> {
    None
}

/// Whether `app` can be started from here.
pub fn installed(app: AppId) -> bool {
    matches!(find(app), Ok(Some(_)))
}

/// The `.app` a macOS binary sits in.
fn bundle_of(binary: &Path) -> Option<&Path> {
    let bundle = binary.parent()?.parent()?.parent()?;
    (cfg!(target_os = "macos") && bundle.extension().is_some_and(|ext| ext == "app")).then_some(bundle)
}

/// Starts `app` with `files`; a running one takes them.
pub fn open(app: AppId, files: &[PathBuf]) -> io::Result<()> {
    let found = find(app)?.ok_or_else(|| {
        ely_ipc::Worded::error(io::ErrorKind::NotFound, "error.not_installed", &[("app", app.name())])
    })?;
    // Launch Services hands files to a running app.
    if let Some(bundle) = bundle_of(&found) {
        let out = Command::new("open").arg("-a").arg(bundle).args(files).output()?;
        if !out.status.success() {
            let said = String::from_utf8_lossy(&out.stderr).trim().to_owned();
            return Err(io::Error::other(format!("{}: {said}", app.name())));
        }
        log::info!("apps: opened {} with {} files", app.name(), files.len());
        return Ok(());
    }
    let mut command = Command::new(&found);
    command.args(files);
    let child = command.spawn()?;
    log::info!("apps: started {} as {}", app.name(), child.id());
    Ok(())
}

/// The Ely app that opens files ending `.ext`.
pub fn for_extension(ext: &str) -> Option<AppId> {
    const TYPES: &str = include_str!("../../../packaging/types.tsv");
    TYPES.lines().find_map(|line| {
        let mut fields = line.split('\t');
        let (slug, kind) = (fields.next()?, fields.next()?);
        if !kind.eq_ignore_ascii_case(ext) {
            return None;
        }
        AppId::ALL.into_iter().find(|app| app.slug() == slug)
    })
}

/// Opens `path` in its Ely app, else the system's.
pub fn open_file(path: &Path) -> io::Result<()> {
    let ext = path.extension().and_then(|ext| ext.to_str());
    match ext.and_then(for_extension).filter(|app| installed(*app)) {
        Some(app) => open(app, &[path.to_path_buf()]),
        None => open_default(path),
    }
}

/// Whether opening `path` would run code, as Windows judges.
#[cfg(windows)]
pub fn runs_code(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let kind: Vec<u16> = ".".encode_utf16().chain(ext.to_string_lossy().encode_utf16()).chain([0]).collect();
    unsafe { windows_sys::Win32::UI::Shell::AssocIsDangerous(kind.as_ptr()) != 0 }
}

/// Elsewhere copies lack the run bit; quarantine guards macOS.
#[cfg(not(windows))]
pub fn runs_code(_: &Path) -> bool {
    false
}

/// Marks a copy as downloaded: the system warns first.
#[cfg(windows)]
pub fn from_internet(path: &Path) -> io::Result<()> {
    let stream = format!("{}:Zone.Identifier", path.display());
    std::fs::write(stream, "[ZoneTransfer]\r\nZoneId=3\r\n")
}

/// Marks a copy as a mail attachment: Gatekeeper asks first.
#[cfg(target_os = "macos")]
pub fn from_internet(path: &Path) -> io::Result<()> {
    use objc2_foundation::{NSDictionary, NSString, NSURL, NSURLQuarantinePropertiesKey};
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    // LaunchServices' quarantine keys, by their string values.
    let keys = [NSString::from_str("LSQuarantineAgentName"), NSString::from_str("LSQuarantineType")];
    let values = [NSString::from_str("ElyMail"), NSString::from_str("LSQuarantineTypeEmailAttachment")];
    let marks = NSDictionary::from_retained_objects(&[&*keys[0], &*keys[1]], &values);
    unsafe { url.setResourceValue_forKey_error(Some(&marks), NSURLQuarantinePropertiesKey) }
        .map_err(|error| io::Error::other(error.localizedDescription().to_string()))
}

/// Linux has no such mark; copies lack the run bit.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn from_internet(_: &Path) -> io::Result<()> {
    Ok(())
}

/// Opens `path` in the default app; a refusal says why.
#[cfg(windows)]
pub fn open_default(path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::UI::{Shell, WindowsAndMessaging::SW_SHOWNORMAL};
    // No shell parses it: a name is never a command.
    let file: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
    let verb: Vec<u16> = "open".encode_utf16().chain([0]).collect();
    let mut info: Shell::SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<Shell::SHELLEXECUTEINFOW>() as u32;
    info.fMask = Shell::SEE_MASK_FLAG_NO_UI | Shell::SEE_MASK_NOASYNC;
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.nShow = SW_SHOWNORMAL;
    if unsafe { Shell::ShellExecuteExW(&mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    log::info!("apps: the system opened an output");
    Ok(())
}

/// Opens `path` in the default app; a refusal says why.
#[cfg(not(windows))]
pub fn open_default(path: &Path) -> io::Result<()> {
    let mut command = match cfg!(target_os = "macos") {
        true => Command::new("open"),
        false => Command::new("xdg-open"),
    };
    let out = ely_ipc::windowless(&mut command).arg(path).output()?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        return Err(io::Error::other(said));
    }
    log::info!("apps: the system opened an output");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::AppId;

    #[cfg(target_os = "macos")]
    #[test]
    fn a_missing_output_says_so() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let error = super::open_default(&dir.path().join("gone.txt")).expect_err("no file");
        assert!(error.to_string().contains("does not exist"), "{error}");
    }

    #[cfg(windows)]
    #[test]
    fn a_file_name_is_never_a_command() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let marker = std::env::current_dir().expect("a folder").join("ely-open-probe");
        // No spaces: unquoted, cmd would run the rest.
        let named = dir.path().join("missing&mkdir,ely-open-probe&rem");
        let opened = super::open_default(&named);
        let ran = marker.exists();
        std::fs::remove_dir(&marker).ok();
        assert!(!ran, "the name ran as a command");
        assert!(opened.is_err(), "a missing file is no success");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_mail_copy_is_quarantined() {
        use std::os::unix::ffi::OsStrExt as _;
        let dir = tempfile::tempdir().expect("a temp dir");
        let file = dir.path().join("run.command");
        std::fs::write(&file, "echo hi").expect("written");
        super::from_internet(&file).expect("marked");
        let path = std::ffi::CString::new(file.as_os_str().as_bytes()).expect("a C path");
        let name = c"com.apple.quarantine";
        let size = unsafe { libc::getxattr(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0, 0, 0) };
        assert!(size > 0, "no quarantine mark");
    }

    #[cfg(windows)]
    #[test]
    fn windows_knows_which_files_run_code() {
        assert!(super::runs_code(Path::new("C:/x/run.exe")));
        assert!(super::runs_code(Path::new("C:/x/Run.BAT")));
        assert!(!super::runs_code(Path::new("C:/x/plan.pdf")));
        assert!(!super::runs_code(Path::new("C:/x/no-kind")));
    }

    #[test]
    fn the_types_table_names_each_kinds_app() {
        assert_eq!(super::for_extension("DOCX"), Some(AppId::Doc));
        assert_eq!(super::for_extension("csv"), Some(AppId::Table));
        assert_eq!(super::for_extension("pdf"), Some(AppId::Pdf));
        assert_eq!(super::for_extension("markdown"), Some(AppId::Mark));
        assert_eq!(super::for_extension("zip"), None);
    }

    #[test]
    fn each_install_layout_names_its_sibling() {
        let suffix = std::env::consts::EXE_SUFFIX;
        let places = super::beside(
            Path::new("/Apps/ElyTree.app/Contents/MacOS/ely-tree"),
            AppId::Mark,
        );
        assert!(places.contains(&PathBuf::from(format!(
            "/Apps/ElyMark.app/Contents/MacOS/ely-mark{suffix}"
        ))));
        let places = super::beside(Path::new("/target/debug/ely-tree"), AppId::Mark);
        assert_eq!(
            places[0],
            PathBuf::from(format!("/target/debug/ely-mark{suffix}"))
        );
        let places = super::beside(Path::new("/Ely Office/ElyTree/ely-tree.exe"), AppId::Mark);
        assert!(places.contains(&PathBuf::from(format!(
            "/Ely Office/ElyMark/ely-mark{suffix}"
        ))));
        let places = super::beside(Path::new("/opt/ely-office/ely-tree/bin/ely-tree"), AppId::Mark);
        assert!(places.contains(&PathBuf::from(format!(
            "/opt/ely-office/ely-mark/bin/ely-mark{suffix}"
        ))));
    }
}
