//! A worker's stderr, carried into the app's log.

use std::{
    io::{BufRead as _, BufReader, Read},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

/// The relaying thread, and what the worker said last.
pub struct Relay {
    last: Arc<Mutex<Said>>,
    thread: Option<JoinHandle<()>>,
}

/// The worker's last error record and its last line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Said {
    pub error: Option<String>,
    /// Raw; may hold engine text once documents are open.
    pub line: Option<String>,
}

impl Relay {
    /// After the worker exits: what it said last.
    pub fn said(&mut self) -> Said {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.last.lock().expect("the relay lock").clone()
    }
}

/// Logs the worker's own records; engine text stays out.
pub fn relay(stderr: impl Read + Send + 'static, who: &'static str) -> Relay {
    let last = Arc::new(Mutex::new(Said::default()));
    let kept = Arc::clone(&last);
    let thread = std::thread::spawn(move || {
        let mut withheld = 0u64;
        let (mut reader, mut bytes) = (BufReader::new(stderr), Vec::new());
        loop {
            bytes.clear();
            match reader.read_until(b'\n', &mut bytes) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) => return log::warn!(target: who, "stderr unreadable: {error}"),
            }
            // Engines may print other encodings; keep reading past them.
            let line = String::from_utf8_lossy(&bytes);
            let line = line.trim_end_matches(['\n', '\r']);
            let record = parse(line);
            {
                let mut said = kept.lock().expect("the relay lock");
                said.line = Some(line.to_string());
                if let Some((log::Level::Error, _, message)) = record {
                    said.error = Some(message.to_string());
                }
            }
            match (record, kernel(line)) {
                (Some((level, target, message)), _) => {
                    log::log!(target: target, level, "{message}")
                }
                (None, Some((level, area, file, number, message))) => {
                    log::log!(target: who, level, "{area} ({file}:{number}): {message}")
                }
                // A crash stack names code, never documents.
                (None, None) if frame(line) => log::error!(target: who, "{line}"),
                (None, None) => {
                    withheld += 1;
                    log::debug!(target: who, "native output withheld ({withheld} lines)");
                }
            }
        }
    });
    Relay { last, thread: Some(thread) }
}

/// `[time LEVEL target] message`, as env_logger writes it.
fn parse(line: &str) -> Option<(log::Level, &str, &str)> {
    let (head, message) = line.strip_prefix('[')?.split_once("] ")?;
    let mut words = head.split_whitespace();
    let (_, level, target) = (words.next()?, words.next()?, words.next()?);
    Some((level.parse().ok()?, target, message))
}

/// `warn:ely.area:pid:tid:path:line: message`, from kernel patch 0100.
fn kernel(line: &str) -> Option<(log::Level, &str, &str, &str, &str)> {
    let (level, rest) = line.split_once(":ely.")?;
    let level = match level {
        "info" => log::Level::Info,
        "warn" => log::Level::Warn,
        _ => return None,
    };
    let mut fields = rest.splitn(4, ':');
    let (area, pid, tid, rest) = (fields.next()?, fields.next()?, fields.next()?, fields.next()?);
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(pid) || !digits(tid) {
        return None;
    }
    // The line number ends the path; drives add colons.
    rest.match_indices(": ").find_map(|(at, _)| {
        let (path, number) = rest[..at].rsplit_once(':')?;
        let file = path.rsplit(['/', '\\']).next()?;
        digits(number).then_some((level, area, file, number, &rest[at + 2..]))
    })
}

/// `3   ely-lok-worker   0x0001 symbol + 12` from backtrace_symbols_fd.
fn frame(line: &str) -> bool {
    let mut words = line.split_whitespace();
    words.next().is_some_and(|n| n.parse::<u32>().is_ok())
        && words.next().is_some()
        && words.next().is_some_and(|address| address.starts_with("0x"))
}

#[cfg(test)]
mod tests {
    use super::{frame, kernel, parse};

    #[test]
    fn a_record_keeps_its_level_and_target() {
        let line = "[2026-10-08T12:00:00Z WARN  ely_lok::worker] request failed";
        assert_eq!(parse(line), Some((log::Level::Warn, "ely_lok::worker", "request failed")));
        assert_eq!(parse("warning: syntax error: unknown keyword: 'Zq'"), None);
    }

    #[test]
    fn a_line_in_another_encoding_stops_nothing() {
        let stderr: &[u8] = b"caf\xe9 opened\n[2026-10-08T12:00:00Z ERROR ely_pdf::worker] it broke\n";
        let mut relay = super::relay(stderr, "test");
        assert_eq!(relay.said().error.as_deref(), Some("it broke"));
    }

    #[test]
    fn the_last_line_is_kept_even_when_it_is_no_record() {
        let stderr: &[u8] = b"[2026-11-03T05:09:10Z ERROR ely_lok::worker] early\r\nfatal runtime error: aborting\r\n";
        let mut relay = super::relay(stderr, "test");
        let said = relay.said();
        assert_eq!(said.error.as_deref(), Some("early"));
        assert_eq!(said.line.as_deref(), Some("fatal runtime error: aborting"));
    }

    #[test]
    fn a_patch_log_keeps_its_area_place_and_message() {
        let windows = r"warn:ely.sc.ui:4242:7:C:\lo\sc\source\ui\docshell\externalrefmgr.cxx:2600: external source not read: C:\a b\x.xlsx: 1";
        let message = r"external source not read: C:\a b\x.xlsx: 1";
        assert_eq!(kernel(windows), Some((log::Level::Warn, "sc.ui", "externalrefmgr.cxx", "2600", message)));
        let unix = "info:ely.writerfilter:9:140245:sw/source/writerfilter/filter/WriterFilter.cxx:310: stays a field";
        assert_eq!(kernel(unix), Some((log::Level::Info, "writerfilter", "WriterFilter.cxx", "310", "stays a field")));
    }

    #[test]
    fn only_patch_logs_pass() {
        let others = [
            "warn:sc.ui:1:2:sc/x.cxx:3: an upstream warning",
            "debug:1:2: temporary output",
            "warn:ely.sc.ui:1:2:no place here",
            "error:ely.sc.ui:1:2:x.cxx:3: no such level",
            "warn:ely.sc.ui:pid:2:x.cxx:3: no process number",
            "a cell's text warn:ely.sc.ui:1:2:x.cxx:3: m",
        ];
        for line in others {
            assert_eq!(kernel(line), None, "{line}");
        }
    }

    static HEARD: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

    struct Heard;

    impl log::Log for Heard {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            let line = format!("{} {} {}", record.level(), record.target(), record.args());
            HEARD.lock().expect("the heard log").push(line);
        }

        fn flush(&self) {}
    }

    #[test]
    fn a_patch_log_reaches_the_app_log_and_upstream_lines_do_not() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            log::set_logger(&Heard).expect("no other logger here");
            log::set_max_level(log::LevelFilter::Debug);
        });
        let stderr: &[u8] = b"warn:sc.ui:1:2:sc/x.cxx:3: upstream text\r\ninfo:ely.sc.ui:1:2:sc/x.cxx:3: read 5 bytes\r\n";
        super::relay(stderr, "patch-test").said();
        let heard: Vec<_> = HEARD.lock().expect("the heard log").iter().filter(|line| line.contains("patch-test")).cloned().collect();
        assert_eq!(heard, ["DEBUG patch-test native output withheld (1 lines)", "INFO patch-test sc.ui (x.cxx:3): read 5 bytes"]);
    }

    #[test]
    fn stack_frames_pass_and_engine_text_does_not() {
        assert!(frame("3   ely-lok-worker    0x0000000104f3c960 report + 52"));
        assert!(!frame("syntax error: unknown keyword: 'ZqxPrivate42'"));
    }
}
