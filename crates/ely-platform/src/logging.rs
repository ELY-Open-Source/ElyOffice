use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    panic,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{Context, Result};
use env_logger::{Builder, Env, Logger, Target};
use log::{Level, Log, Metadata, Record};

use crate::{AppId, data_dir};

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

struct Tee(File);

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write_all(buf)?;
        io::stderr().write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()?;
        io::stderr().flush()
    }
}

/// The HTTP library logs raw bytes, tokens too.
struct Private(Logger);

fn wire(meta: &Metadata) -> bool {
    let target = meta.target();
    let http = target == "ureq" || target.starts_with("ureq::") || target.starts_with("ureq_proto");
    http && meta.level() > Level::Info
}

/// html5ever says foster parenting is missing; it is not.
fn stale(record: &Record) -> bool {
    record.target() == "html5ever::tree_builder"
        && record.args().as_str() == Some("foster parenting not implemented")
}

impl Log for Private {
    fn enabled(&self, meta: &Metadata) -> bool {
        !wire(meta) && self.0.enabled(meta)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) && !stale(record) {
            self.0.log(record);
        }
    }

    fn flush(&self) {
        self.0.flush();
    }
}

/// The file logging writes to; none until `init` succeeds.
pub fn path() -> Option<&'static Path> {
    LOG_PATH.get().map(PathBuf::as_path)
}

/// Past this a log keeps only its newer half (§18.3).
const LOG_CAP: u64 = 8 * 1024 * 1024;

/// Keeps a long log's newer half, from a whole line.
fn trimmed(path: &Path, cap: u64) -> std::io::Result<()> {
    let length = match fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if length <= cap {
        return Ok(());
    }
    let bytes = fs::read(path)?;
    let tail = &bytes[(length - cap / 2) as usize..];
    let whole = tail.iter().position(|b| *b == b'\n').map_or(tail.len(), |n| n + 1);
    fs::write(path, &tail[whole..])
}

/// Milliseconds in each stamp: the bench times by them.
fn built(builder: &mut Builder, target: Target) -> Logger {
    builder.format_timestamp_millis().target(target).build()
}

/// Logs to stderr and a file. `ELY_LOG` sets level.
pub fn init(app: AppId) -> Result<()> {
    init_in(app, &data_dir(app)?)
}

pub fn init_in(app: AppId, dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let path = dir.join("ely.log");
    trimmed(&path, LOG_CAP).with_context(|| format!("cannot trim {}", path.display()))?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("cannot open {}", path.display()))?;
    let logger = built(
        &mut Builder::from_env(Env::new().filter_or("ELY_LOG", "info")),
        Target::Pipe(Box::new(Tee(file))),
    );
    let level = logger.filter();
    log::set_boxed_logger(Box::new(Private(logger))).context("a logger is already set")?;
    log::set_max_level(level);
    LOG_PATH.set(path).expect("the logger was set once");

    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        log::error!("panic: {info}");
        previous(info);
    }));
    log::info!("{} {} started", app.name(), env!("CARGO_PKG_VERSION"));
    Ok(())
}

#[cfg(test)]
mod trim_tests {
    #[test]
    fn a_long_log_keeps_its_newer_half_from_a_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ely.log");
        let lines: String = (0..200).map(|n| format!("line {n:03}\n")).collect();
        std::fs::write(&path, &lines).unwrap();
        super::trimmed(&path, 1000).unwrap();
        let kept = std::fs::read_to_string(&path).unwrap();
        assert!(kept.len() <= 500 && kept.starts_with("line ") && kept.ends_with("line 199\n"), "{kept}");
        super::trimmed(&path, 1000).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), kept, "a short log stays");
    }
}

#[cfg(test)]
mod filter_tests {
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };

    use env_logger::{Builder, Target};
    use log::{Level, Log, Record};

    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<u8>>>);

    impl Write for Shared {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn logged(target: &str, level: Level, args: std::fmt::Arguments) -> String {
        let out = Shared::default();
        let logger = super::built(
            Builder::new().filter_level(log::LevelFilter::Trace),
            Target::Pipe(Box::new(out.clone())),
        );
        let record = Record::builder()
            .target(target)
            .level(level)
            .args(args)
            .build();
        super::Private(logger).log(&record);
        String::from_utf8(out.0.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn a_stale_html5ever_claim_stays_out_and_others_stay_in() {
        let tree = "html5ever::tree_builder";
        let stale = logged(
            tree,
            Level::Warn,
            format_args!("foster parenting not implemented"),
        );
        assert!(stale.is_empty(), "{stale}");
        let other = logged(tree, Level::Warn, format_args!("something else"));
        assert!(other.contains("something else"), "{other}");
        let wire = logged("ureq::unit", Level::Debug, format_args!("bytes"));
        assert!(wire.is_empty(), "{wire}");
    }

    #[test]
    fn a_line_is_stamped_to_the_millisecond() {
        let line = logged("ely", Level::Info, format_args!("ready"));
        let stamp = line.trim_start_matches('[').split(' ').next().unwrap();
        let fraction = stamp.trim_end_matches('Z').rsplit_once('.').map(|(_, ms)| ms);
        assert!(fraction.is_some_and(|ms| ms.len() == 3 && ms.bytes().all(|b| b.is_ascii_digit())), "{line}");
    }
}
