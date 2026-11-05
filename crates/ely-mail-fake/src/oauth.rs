//! A fake token endpoint and browser for OAuth tests.

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ely_mail_engine::oauth::{Provider, SignIn, Tokens};

/// Answers each request in turn; reports each form.
pub fn endpoint(answers: Vec<(u16, String)>) -> (String, mpsc::Receiver<HashMap<String, String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/token", listener.local_addr().unwrap());
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for (status, body) in answers {
            let (mut stream, _) = listener.accept().unwrap();
            // Tests that ignore the forms drop the receiver.
            sender.send(form(&mut stream)).ok();
            let reply = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
        }
    });
    (url, receiver)
}

fn form(stream: &mut TcpStream) -> HashMap<String, String> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0, "the request ended early");
        data.extend_from_slice(&chunk[..count]);
        let text = String::from_utf8_lossy(&data).to_string();
        if let Some((head, rest)) = text.split_once("\r\n\r\n") {
            let length: usize = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse().unwrap())
                })
                .unwrap();
            if rest.len() >= length {
                return form_urlencoded::parse(rest.as_bytes()).into_owned().collect();
            }
        }
    }
}

pub fn provider(token: String) -> Provider {
    Provider {
        name: "test",
        authorize: "https://auth.example/authorize".into(),
        token,
        client_id: "client-1".into(),
        scope: "mail openid email".into(),
        extra: &[("prompt", "consent")],
        redirect_host: "127.0.0.1",
    }
}

pub fn id_token(claims: &str) -> String {
    format!("e30.{}.sig", URL_SAFE_NO_PAD.encode(claims))
}

pub fn query(url: &str) -> HashMap<String, String> {
    let (_, query) = url.split_once('?').unwrap();
    form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect()
}

/// The browser following the redirect; returns the status line.
pub fn browse(redirect: &str, path: &str) -> String {
    let address = redirect.trim_start_matches("http://");
    let mut stream = TcpStream::connect(address).unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n").unwrap();
    let mut reply = String::new();
    stream.read_to_string(&mut reply).unwrap();
    reply.lines().next().unwrap().to_owned()
}

/// The browser coming back to the page `url` opened.
pub fn approve(url: &str) -> thread::JoinHandle<String> {
    let asked = query(url);
    let (redirect, state) = (asked["redirect_uri"].clone(), asked["state"].clone());
    thread::spawn(move || browse(&redirect, &format!("/?code=c&state={state}")))
}

/// Completes a sign-in whose endpoint answers with body.
pub async fn signed_in(body: String) -> ely_mail_engine::Result<Tokens> {
    let (token, _) = endpoint(vec![(200, body)]);
    let sign_in = SignIn::start(provider(token), "").await.unwrap();
    let browser = approve(sign_in.url());
    let result = sign_in.finish().await;
    browser.join().unwrap();
    result
}
