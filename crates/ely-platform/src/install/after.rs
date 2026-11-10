//! What runs once the app has quit: install, then reopen.

use std::{
    io,
    path::Path,
    process::{Child, Command, Stdio},
};

use super::Installed;

/// The argument that runs the app as the macOS installer.
pub const INSTALL: &str = "--ely-install-update";

/// Waits for this process, installs, reopens; failures go in `note`.
const WINDOWS: &str = r#"
$ErrorActionPreference = 'Stop'
Get-Process -Id $env:ELY_PID -ErrorAction SilentlyContinue | Wait-Process
$said = $null
try {
  # Exit 2: a file in use, perhaps a lingering worker.
  for ($try = 1; ; $try++) {
    $setup = Start-Process -FilePath $env:ELY_SETUP -ArgumentList '/S', ('/D=' + $env:ELY_DIR) -Wait -PassThru
    if ($setup.ExitCode -ne 2 -or $try -ge 20) { break }
    Start-Sleep -Seconds 1
  }
  if ($setup.ExitCode -ne 0) { $said = "the setup exited with $($setup.ExitCode) (2: a file is in use)" }
} catch { $said = "the setup did not run: $_" }
if ($said) {
  [Console]::Error.WriteLine("update: $said")
  # An unwritten note must not keep the app closed.
  try {
    New-Item -ItemType Directory -Force -Path (Split-Path -LiteralPath $env:ELY_NOTE) | Out-Null
    [IO.File]::WriteAllText($env:ELY_NOTE, $said)
  } catch { [Console]::Error.WriteLine("update: the note was not written: $_") }
}
Start-Process -FilePath $env:ELY_EXE
"#;

/// The same for an AppImage: swap the file, then reopen.
const APPIMAGE: &str = r#"
pid=$1 new=$2 target=$3 note=$4
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
staged="$(dirname "$target")/.$(basename "$target").new"
if ! said=$({ cp "$new" "$staged" && chmod 755 "$staged" && mv -f "$staged" "$target"; } 2>&1); then
  rm -f "$staged"
  echo "update: $target was not replaced: $said" >&2
  mkdir -p "$(dirname "$note")" && printf '%s\n' "$target was not replaced: $said" >"$note"
fi
setsid "$target" </dev/null >/dev/null 2>&1 &
"#;

/// Starts what installs `file` after process `pid` quits.
pub fn after_quit(file: &Path, installed: &Installed, pid: u32, note: &Path, said: Stdio) -> io::Result<Child> {
    let mut command = match installed {
        Installed::Bundle(bundle) => {
            let mut command = Command::new(std::env::current_exe()?);
            command.arg(INSTALL).arg(file).arg(bundle).arg(pid.to_string());
            command
        }
        Installed::Folder { dir, exe } => {
            let mut command = Command::new("powershell.exe");
            command
                .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", WINDOWS])
                .env("ELY_PID", pid.to_string())
                .env("ELY_SETUP", file)
                .env("ELY_DIR", dir)
                .env("ELY_EXE", exe)
                .env("ELY_NOTE", note);
            command
        }
        Installed::AppImage(image) => {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", APPIMAGE, "sh"]).arg(pid.to_string()).arg(file).arg(image).arg(note);
            command
        }
        Installed::Package(..) => {
            return Err(io::Error::new(io::ErrorKind::Unsupported, "the package manager installs this app"));
        }
    };
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(said);
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    ely_ipc::windowless(&mut command).spawn()
}
