//! Mail and sign-in go through the named proxy (#35).

use std::sync::{Arc, Mutex};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ely_mail_engine::{Imap, Result, Secret, Security, fetch, oauth};
use fake::{certs, expect, login, send, serve};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
};

/// What each proxy was asked, line by line.
type Asked = Arc<Mutex<Vec<String>>>;

async fn listen() -> (TcpListener, u16) {
    listen_on("127.0.0.1:0").await
}

async fn listen_on(address: &str) -> (TcpListener, u16) {
    let listener = TcpListener::bind(address).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, port)
}

async fn head(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await.unwrap());
    }
    String::from_utf8(head).unwrap()
}

/// A CONNECT proxy giving `answer`; keeps each ask and login.
async fn http_proxy(answer: &'static str) -> (u16, Asked) {
    http_proxy_on("127.0.0.1:0", answer).await
}

async fn http_proxy_on(address: &str, answer: &'static str) -> (u16, Asked) {
    let (listener, port) = listen_on(address).await;
    let asked = Asked::default();
    let seen = asked.clone();
    tokio::spawn(async move {
        loop {
            let (mut client, _) = listener.accept().await.unwrap();
            let head = head(&mut client).await;
            let line = head.lines().next().unwrap().to_owned();
            let target = line.split(' ').nth(1).unwrap().to_owned();
            seen.lock().unwrap().push(line);
            let login = head.lines().find(|line| line.starts_with("Proxy-Authorization: "));
            seen.lock().unwrap().extend(login.map(str::to_owned));
            client.write_all(answer.as_bytes()).await.unwrap();
            if answer.starts_with("HTTP/1.1 200") {
                let mut server = TcpStream::connect(target).await.unwrap();
                tokio::spawn(async move { copy_bidirectional(&mut client, &mut server).await });
            }
        }
    });
    (port, asked)
}

async fn field(client: &mut TcpStream) -> String {
    let length = client.read_u8().await.unwrap() as usize;
    let mut text = vec![0u8; length];
    client.read_exact(&mut text).await.unwrap();
    String::from_utf8(text).unwrap()
}

/// A SOCKS5 proxy; with `login`, it asks for one.
async fn socks_proxy(login: bool) -> (u16, Asked) {
    let (listener, port) = listen().await;
    let asked = Asked::default();
    let seen = asked.clone();
    tokio::spawn(async move {
        loop {
            let (mut client, _) = listener.accept().await.unwrap();
            let mut hello = [0u8; 2];
            client.read_exact(&mut hello).await.unwrap();
            let mut methods = vec![0u8; hello[1] as usize];
            client.read_exact(&mut methods).await.unwrap();
            if login {
                client.write_all(&[5, 2]).await.unwrap();
                assert_eq!(client.read_u8().await.unwrap(), 1, "RFC 1929");
                let user = field(&mut client).await;
                let password = field(&mut client).await;
                seen.lock().unwrap().push(format!("{user}:{password}"));
                client.write_all(&[1, 0]).await.unwrap();
            } else {
                client.write_all(&[5, 0]).await.unwrap();
            }
            let mut ask = [0u8; 4];
            client.read_exact(&mut ask).await.unwrap();
            assert_eq!(ask, [5, 1, 0, 3], "a CONNECT by name");
            let name = field(&mut client).await;
            let port = client.read_u16().await.unwrap();
            let target = format!("{name}:{port}");
            seen.lock().unwrap().push(target.clone());
            client.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await.unwrap();
            let mut server = TcpStream::connect(target).await.unwrap();
            tokio::spawn(async move { copy_bidirectional(&mut client, &mut server).await });
        }
    });
    (port, asked)
}

/// One IMAP login and logout; the server's port.
async fn log_in() -> (u16, Result<()>) {
    let certs = certs();
    let (server, _) = serve(
        &certs,
        Security::Tls,
        vec![
            send("* OK ready\n"),
            expect("{tag} LOGIN \"ann@example.com\" \"pw\""),
            send("{tag} OK done\n"),
            expect("{tag} LOGOUT"),
            send("* BYE\n{tag} OK bye\n"),
        ],
    )
    .await;
    let logged = match Imap::connect(&server, &login(Secret::Password("pw".into())), &certs.trusted).await {
        Ok(imap) => imap.logout().await,
        Err(err) => Err(err),
    };
    (server.port, logged)
}

/// A web server giving `reply` once, after reading the ask.
async fn web_server(reply: String) -> u16 {
    let (listener, port) = listen().await;
    tokio::spawn(async move {
        let (mut client, _) = listener.accept().await.unwrap();
        let asked = head(&mut client).await;
        let length = asked
            .lines()
            .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length: ").map(str::to_owned))
            .map_or(0, |length| length.parse().unwrap());
        let mut body = vec![0u8; length];
        client.read_exact(&mut body).await.unwrap();
        client.write_all(reply.as_bytes()).await.unwrap();
    });
    port
}

fn ok(kind: &str, body: &str) -> String {
    format!("HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn provider(token: String) -> oauth::Provider {
    oauth::Provider {
        name: "test",
        authorize: String::new(),
        token,
        client_id: "id".into(),
        scope: String::new(),
        extra: &[],
        redirect_host: "127.0.0.1",
    }
}

fn named(proxy: &str, skip: Option<&str>) {
    // SAFETY: this binary's only test; no reader meanwhile.
    unsafe {
        for name in ["ALL_PROXY", "all_proxy", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "NO_PROXY", "no_proxy"] {
            std::env::remove_var(name);
        }
        std::env::set_var("ALL_PROXY", proxy);
        if let Some(skip) = skip {
            std::env::set_var("NO_PROXY", skip);
        }
    }
}

#[tokio::test]
async fn mail_and_sign_in_go_through_the_named_proxy() {
    let (port, asked) = http_proxy("HTTP/1.1 200 Connection established\r\n\r\n").await;
    named(&format!("http://127.0.0.1:{port}"), None);
    let (server, logged) = log_in().await;
    logged.unwrap();
    assert_eq!(*asked.lock().unwrap(), [format!("CONNECT 127.0.0.1:{server} HTTP/1.1")]);

    let json = r#"{"access_token":"a-1","expires_in":60}"#;
    let token = web_server(ok("application/json", json)).await;
    let tokens = oauth::refresh(&provider(format!("http://127.0.0.1:{token}/token")), "r-1").await.unwrap();
    assert_eq!(tokens.access, "a-1");
    assert_eq!(asked.lock().unwrap()[1], format!("CONNECT 127.0.0.1:{token} HTTP/1.1"));

    // Each redirect is asked of the proxy again.
    let picture = web_server(ok("image/png", "png")).await;
    let moved = format!("HTTP/1.1 302 Found\r\nLocation: http://localhost:{picture}/p.png\r\nContent-Length: 0\r\n\r\n");
    let first = web_server(moved).await;
    named(&format!("http://127.0.0.1:{port}"), Some("127.0.0.1"));
    let url = format!("http://127.0.0.1:{first}/p.png");
    let (kind, bytes) = tokio::task::spawn_blocking(move || fetch::picture(&url)).await.unwrap().unwrap();
    assert_eq!((kind.as_str(), bytes.as_slice()), ("image/png", &b"png"[..]));
    assert_eq!(asked.lock().unwrap()[2], format!("CONNECT localhost:{picture} HTTP/1.1"));

    let (port, asked) = http_proxy("HTTP/1.1 200 Connection established\r\n\r\n").await;
    named(&format!("http://ann:pa%3As%40s@127.0.0.1:{port}"), None);
    log_in().await.1.unwrap();
    let login = format!("Proxy-Authorization: Basic {}", STANDARD.encode("ann:pa:s@s"));
    assert_eq!(asked.lock().unwrap()[1], login);
    let err = oauth::refresh(&provider("http://127.0.0.1:1/token".into()), "r-1").await.err().unwrap();
    assert!(err.to_string().contains("a proxy login with escapes"), "{err}");

    let (port, asked) = socks_proxy(false).await;
    named(&format!("socks5://127.0.0.1:{port}"), None);
    let (server, logged) = log_in().await;
    logged.unwrap();
    assert_eq!(*asked.lock().unwrap(), [format!("127.0.0.1:{server}")]);

    let (port, asked) = socks_proxy(true).await;
    named(&format!("socks5://ann:pa%3Ass@127.0.0.1:{port}"), None);
    let (server, logged) = log_in().await;
    logged.unwrap();
    assert_eq!(*asked.lock().unwrap(), ["ann:pa:ss".to_owned(), format!("127.0.0.1:{server}")]);

    let (port, asked) = http_proxy("HTTP/1.1 407 Proxy Authentication Required\r\n\r\n").await;
    named(&format!("http://127.0.0.1:{port}"), None);
    let err = log_in().await.1.unwrap_err().to_string();
    assert!(err.contains("407 Proxy Authentication Required"), "{err}");

    named(&format!("http://127.0.0.1:{port}"), Some("127.0.0.1"));
    log_in().await.1.unwrap();
    assert_eq!(asked.lock().unwrap().len(), 1, "a bypassed host goes straight");

    // A setting that does not parse never means straight.
    named("http://[broken", None);
    let token = web_server(ok("application/json", json)).await;
    let err = oauth::refresh(&provider(format!("http://127.0.0.1:{token}/token")), "r-1").await.err().unwrap();
    assert!(err.to_string().contains("proxy settings unreadable: ALL_PROXY"), "{err}");
    let err = log_in().await.1.unwrap_err().to_string();
    assert!(err.contains("proxy settings unreadable: ALL_PROXY"), "{err}");

    let (port, asked) = http_proxy_on("[::1]:0", "HTTP/1.1 200 Connection established\r\n\r\n").await;
    named(&format!("http://[::1]:{port}"), None);
    let (server, logged) = log_in().await;
    logged.unwrap();
    assert_eq!(*asked.lock().unwrap(), [format!("CONNECT 127.0.0.1:{server} HTTP/1.1")]);

    #[cfg(windows)]
    let garbled = <std::ffi::OsString as std::os::windows::ffi::OsStringExt>::from_wide(&[0xD800]);
    #[cfg(unix)]
    let garbled = <std::ffi::OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![0xff]);
    // SAFETY: this binary's only test; no reader meanwhile.
    unsafe { std::env::set_var("ALL_PROXY", garbled) };
    let err = log_in().await.1.unwrap_err().to_string();
    assert!(err.contains("ALL_PROXY is not text"), "{err}");
}
