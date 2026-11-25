//! The command line: verbs, standard input, exit codes (#107).

use std::{
    io::{self, Read as _, Write as _},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use ely_platform::{
    AppId,
    instance::{self, Launch},
};

/// The most a request may carry, as the door takes.
const LIMIT: usize = 64 * 1024 * 1024;

/// How long a started app has to open its file.
const START: Duration = Duration::from_secs(60);

/// Words that are verbs, not files to open.
const VERBS: [&str; 7] = ["docs", "open", "new", "export", "save", "help", "--help"];

/// A noun's words as sent; files made absolute.
pub type Request = fn(&[&str]) -> Result<Vec<String>, String>;

/// What one app's command line knows.
pub struct Cli {
    pub app: AppId,
    /// The app's nouns, as `cell`, and each one's request.
    pub nouns: &'static [(&'static str, Request)],
    pub usage: &'static str,
    /// Noun verbs whose answer prints as is.
    pub exact: &'static [&'static str],
    /// What `new` writes, by extension.
    pub blanks: &'static [(&'static str, &'static [u8])],
    /// What `export` writes, by extension.
    pub exports: &'static [&'static str],
}

/// Why a command ended early, and its exit code.
#[derive(Debug, PartialEq)]
pub(crate) struct Fail {
    code: i32,
    said: String,
}

fn fail(code: i32, said: impl Into<String>) -> Fail {
    Fail { code, said: said.into() }
}

/// What the words ask for.
#[derive(Debug, PartialEq)]
pub(crate) enum Plan {
    Usage,
    /// Sent as is; whether its answer prints exactly.
    Ask(Vec<String>, bool),
    Open(String),
    New(String, &'static [u8]),
}

/// `cli`'s command, or None to start as usual.
pub fn command_line(cli: &Cli) -> Option<i32> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let first = args.first()?.to_str()?;
    if !cli.nouns.iter().any(|(noun, _)| *noun == first) && !VERBS.contains(&first) {
        return None;
    }
    ely_platform::startup::attach_console();
    let done = args
        .iter()
        .map(|arg| arg.to_str().map(String::from).ok_or_else(|| fail(2, "an argument is not text")))
        .collect::<Result<Vec<_>, _>>()
        .and_then(|words| with_stdin(words, read_stdin))
        .and_then(|words| plan(cli, &words))
        .and_then(|plan| run(cli, plan));
    let slug = cli.app.slug();
    Some(match done {
        Ok(printed) => {
            // Exit runs no destructors: flush here.
            let mut out = io::stdout();
            match out.write_all(printed.as_bytes()).and_then(|()| out.flush()) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("{slug}: the answer did not print: {error}");
                    1
                }
            }
        }
        Err(Fail { code: 2, said }) => {
            eprintln!("{slug}: {said}\n{}", cli.usage);
            2
        }
        Err(Fail { code, said }) => {
            eprintln!("{slug}: {said}");
            code
        }
    })
}

fn read_stdin() -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::stdin().take(LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Each `-` becomes standard input, read once.
pub(crate) fn with_stdin(mut words: Vec<String>, read: impl FnOnce() -> io::Result<Vec<u8>>) -> Result<Vec<String>, Fail> {
    let dashes: Vec<usize> = (0..words.len()).filter(|at| words[*at] == "-").collect();
    let Some(&at) = dashes.first() else {
        return Ok(words);
    };
    if dashes.len() > 1 {
        return Err(fail(2, "standard input (-) can be used for only one argument"));
    }
    let bytes = read().map_err(|error| fail(2, format!("standard input: {error}")))?;
    if bytes.len() > LIMIT {
        return Err(fail(2, "standard input exceeds the 64 MiB limit"));
    }
    words[at] = String::from_utf8(bytes).map_err(|_| fail(2, "standard input is not valid UTF-8 text"))?;
    Ok(words)
}

fn absolute(file: &str) -> Result<String, Fail> {
    super::absolute(file).map_err(|why| fail(1, why))
}

/// A path's extension, lower case.
fn extension(file: &str) -> String {
    Path::new(file).extension().map(|ext| ext.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// The words made a plan; nothing is sent yet.
pub(crate) fn plan(cli: &Cli, words: &[String]) -> Result<Plan, Fail> {
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let ask = |words: Vec<String>| Ok(Plan::Ask(words, false));
    match words[..] {
        ["help" | "--help"] => Ok(Plan::Usage),
        ["docs"] => ask(vec!["docs".into()]),
        ["save", file] => ask(vec!["save".into(), absolute(file)?]),
        ["open", file] => Ok(Plan::Open(absolute(file)?)),
        ["new", file] => {
            let ext = extension(file);
            match cli.blanks.iter().find(|(each, _)| *each == ext) {
                Some((_, bytes)) => Ok(Plan::New(absolute(file)?, bytes)),
                None => Err(fail(2, format!("{file}: cannot create this type (supported: {})", listed(cli.blanks.iter().map(|(ext, _)| *ext))))),
            }
        }
        ["export", file, target] => {
            let (file, to) = (absolute(file)?, absolute(target)?);
            if same(&file, &to) {
                return Err(fail(2, "export target cannot be the same as the open file"));
            }
            if !cli.exports.contains(&extension(&to).as_str()) {
                return Err(fail(2, format!("{target}: cannot export to this type (supported: {})", listed(cli.exports.iter().copied()))));
            }
            ask(vec!["export".into(), file, to])
        }
        [noun, ref rest @ ..] if let Some((_, request)) = cli.nouns.iter().find(|(name, _)| *name == noun) => {
            let words = request(rest).map_err(|why| fail(2, why))?;
            let exact = words.first().is_some_and(|verb| cli.exact.contains(&verb.as_str()));
            Ok(Plan::Ask(words, exact))
        }
        _ => Err(fail(2, "unknown command")),
    }
}

fn listed<'a>(extensions: impl Iterator<Item = &'a str>) -> String {
    extensions.map(|ext| format!(".{ext}")).collect::<Vec<_>>().join(", ")
}

/// One file under two names, as the disk tells.
fn same(a: &str, b: &str) -> bool {
    a == b || matches!((dunce::canonicalize(a), dunce::canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

/// Runs `plan`; what prints.
fn run(cli: &Cli, plan: Plan) -> Result<String, Fail> {
    match plan {
        Plan::Usage => Ok(format!("{}\n", cli.usage)),
        Plan::Ask(words, exact) => ask(cli.app, &words).map(|answer| match exact || answer.is_empty() {
            true => answer,
            false => format!("{answer}\n"),
        }),
        Plan::Open(file) => open(cli.app, &file),
        Plan::New(file, blank) => {
            write_new(&file, blank)?;
            log::info!("{}: wrote a blank for the command line", cli.app.slug());
            open(cli.app, &file)
        }
    }
}

/// Whether `error` means no copy is running.
fn not_running(error: &io::Error) -> bool {
    matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused)
}

fn ask(app: AppId, words: &[String]) -> Result<String, Fail> {
    let request = serde_json::Value::from(words).to_string();
    if request.len() > LIMIT {
        return Err(fail(2, "standard input exceeds the 64 MiB limit"));
    }
    instance::ask(app, &request).map_err(|error| match not_running(&error) {
        true => fail(3, format!("{} is not running; use open FILE to start it", app.name())),
        false => fail(1, error.to_string()),
    })
}

/// A new file holding `blank`; one already there stays.
pub(crate) fn write_new(file: &str, blank: &[u8]) -> Result<(), Fail> {
    let made = std::fs::OpenOptions::new().write(true).create_new(true).open(file);
    let mut out = match made {
        Ok(out) => out,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Err(fail(1, format!("{file}: file already exists"))),
        Err(error) => return Err(fail(1, format!("could not open {file}: {error}"))),
    };
    out.write_all(blank).and_then(|()| out.sync_all()).map_err(|error| fail(1, format!("could not open {file}: {error}")))
}

/// Opens `file`, starting the app if need be; once loaded.
fn open(app: AppId, file: &str) -> Result<String, Fail> {
    let failed = |error: &dyn std::fmt::Display| fail(1, format!("could not open {file}: {error}"));
    match Path::new(file).try_exists() {
        Ok(true) => {}
        Ok(false) => return Err(failed(&io::Error::from(io::ErrorKind::NotFound))),
        Err(error) => return Err(failed(&error)),
    }
    let launch = Launch { files: vec![file.into()], new_window: false };
    match instance::hand(app, &launch) {
        Ok(()) => log::info!("{}: the command line handed a file over", app.slug()),
        Err(error) if not_running(&error) => start(app, file).map_err(|error| failed(&error))?,
        Err(error) => return Err(failed(&error)),
    }
    let request = serde_json::json!(["open", file]).to_string();
    let deadline = Instant::now() + START;
    loop {
        match instance::ask(app, &request) {
            Ok(answer) => return Ok(format!("{answer}\n")),
            Err(error) if not_running(&error) && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(200)),
            Err(error) if not_running(&error) => return Err(fail(1, format!("did not start or finish loading within {} seconds", START.as_secs()))),
            Err(error) => return Err(failed(&error)),
        }
    }
}

/// Starts `app` on `file`, apart from this terminal.
fn start(app: AppId, file: &str) -> io::Result<()> {
    let mut command = Command::new(std::env::current_exe()?);
    command.arg(file).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, DETACHED_PROCESS};
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let child = command.spawn()?;
    log::info!("{}: the command line started it, pid {}", app.slug(), child.id());
    Ok(())
}

#[cfg(test)]
mod tests;
