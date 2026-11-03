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
            match record {
                Some((level, target, message)) => {
                    log::log!(target: target, level, "{message}")
                }
                // A crash stack names code, never documents.
                None if frame(line) => log::error!(target: who, "{line}"),
                None => {
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

/// `3   ely-lok-worker   0x0001 symbol + 12` from backtrace_symbols_fd.
fn frame(line: &str) -> bool {
    let mut words = line.split_whitespace();
    words.next().is_some_and(|n| n.parse::<u32>().is_ok())
        && words.next().is_some()
        && words.next().is_some_and(|address| address.starts_with("0x"))
}

#[cfg(test)]
mod tests {
    use super::{frame, parse};

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
    fn stack_frames_pass_and_engine_text_does_not() {
        assert!(frame("3   ely-lok-worker    0x0000000104f3c960 report + 52"));
        assert!(!frame("syntax error: unknown keyword: 'ZqxPrivate42'"));
    }
}
