//! A local server and signed manifests for update tests.
#![allow(dead_code)]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use ely_platform::update::{Release, sign};
use semver::Version;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub enum Reply {
    Body(Vec<u8>),
    Redirect(String),
    Stall(Arc<AtomicBool>),
}

/// The test key pair; `tests/scripts/update.sh` signs with it too.
pub const SEED: &str = "JWPqrMuIaO6Rxz6efh5PJFonrCtZcc0yZnLG5pMUmKk=";
pub const KEY: &str = "LWfbar3eMGzvVABTvGC618oPPHvgHL/Cu30umm2w1Xg=";

/// The route of `name`'s signature over `reply`.
pub fn signature_of(name: &str, reply: &Reply) -> (String, Reply) {
    let Reply::Body(bytes) = reply else {
        panic!("only a body is signed");
    };
    let signature = sign(SEED, name, bytes).expect("signed");
    (format!("/{name}.sig"), Reply::Body(signature.into_bytes()))
}

pub fn hex_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn payload() -> Vec<u8> {
    (0..300_000u32).map(|n| (n % 251) as u8).collect()
}

fn respond(mut stream: TcpStream, routes: &[(String, Reply)]) {
    let mut request = Vec::new();
    let mut byte = [0u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).unwrap() == 0 {
            return;
        }
        request.push(byte[0]);
    }
    let line = String::from_utf8_lossy(&request);
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let reply = routes
        .iter()
        .find(|(route, _)| *route == path)
        .map(|(_, reply)| reply.clone());
    let bytes = match reply {
        Some(Reply::Body(body)) => {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            [head.into_bytes(), body].concat()
        }
        Some(Reply::Redirect(to)) => format!(
            "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .into_bytes(),
        Some(Reply::Stall(closed)) => {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n",
            );
            let _ = stream.write_all(&[7u8; 1000]);
            stream
                .set_read_timeout(Some(Duration::from_secs(60)))
                .unwrap();
            let seen = stream.read(&mut [0u8; 1]);
            if matches!(seen, Ok(0))
                || seen.is_err_and(|error| error.kind() == std::io::ErrorKind::ConnectionReset)
            {
                closed.store(true, Ordering::Relaxed);
            }
            return;
        }
        None => {
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
        }
    };
    let _ = stream.write_all(&bytes);
}

/// Serves routes locally; `make` gets the base URL.
pub fn serve(make: impl FnOnce(&str) -> Vec<(String, Reply)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    let routes = make(&base);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let routes = routes.clone();
            let stream = stream.unwrap();
            thread::spawn(move || respond(stream, &routes));
        }
    });
    base
}

pub fn manifest(version: &str, key: &str, base: &str, sha: &str) -> Reply {
    Reply::Body(
        format!(r#"{{"version":"{version}","assets":{{"{key}":{{"url":"{base}/ElyDoc.bin","sha256":"{sha}"}}}}}}"#)
            .into_bytes(),
    )
}

pub fn release(base: &str, sha: String) -> Release {
    Release {
        version: Version::new(0, 2, 0),
        url: format!("{base}/ElyDoc.bin"),
        sha256: sha,
    }
}

pub fn error_text<T>(result: anyhow::Result<T>) -> String {
    format!("{:#}", result.err().expect("an error"))
}

