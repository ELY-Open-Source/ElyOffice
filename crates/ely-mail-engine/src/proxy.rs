//! The proxy the system names, for mail and web (#35).

use std::io;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpStream,
};
use ureq::{Proxy, ProxyProtocol, http::Uri};

use crate::{Error, Result};

/// Proxy variables, read in curl's order.
const NAMED: [&str; 6] = ["ALL_PROXY", "all_proxy", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"];

/// The system's own settings never take loopback.
const LOOPBACK: [&str; 4] = ["localhost", "127.", "::1", "[::1]"];

/// A proxy as one setting names it.
struct Named {
    /// The variable or setting it came from.
    from: &'static str,
    address: String,
    /// Hosts that go straight.
    skips: Vec<String>,
    system: bool,
}

/// A proxy, its bypass list inside, and its login decoded.
pub(crate) struct Route {
    proxy: Proxy,
    login: Option<(String, String)>,
}

/// The proxy and its login; None when none is set.
fn current() -> Result<Option<Route>> {
    let named = match env()? {
        Some(named) => Some(named),
        None => system()?,
    };
    let Some(named) = named else {
        return Ok(None);
    };
    let route = build(&named)?;
    let proxy = &route.proxy;
    log::info!("net: the {:?} proxy {}:{} from {}", proxy.protocol(), proxy.host(), proxy.port(), named.from);
    Ok(Some(route))
}

/// The proxy for web requests; ureq picks per request.
pub(crate) fn for_web() -> Result<Option<Proxy>> {
    current()?.map(web).transpose()
}

/// ureq sends the login undecoded; escaped ones are refused.
fn web(route: Route) -> Result<Proxy> {
    let raw = route.proxy.username().map(|user| (user, route.proxy.password().unwrap_or_default()));
    let login = route.login.as_ref().map(|(user, password)| (user.as_str(), password.as_str()));
    match login == raw {
        true => Ok(route.proxy),
        false => Err(unreadable("a proxy login with escapes; web requests send it as written".into())),
    }
}

/// The proxy for `host`; None goes straight.
pub(crate) fn for_host(host: &str) -> Result<Option<Route>> {
    let Some(route) = current()? else {
        return Ok(None);
    };
    let proxy = &route.proxy;
    let uri: Uri = format!("http://{}/", authority(host, 1))
        .parse()
        .map_err(|err| unreadable(format!("{host}: {err}")))?;
    if proxy.is_no_proxy(&uri) {
        log::info!("net: {host} is bypassed; straight");
        return Ok(None);
    }
    Ok(Some(route))
}

fn unreadable(why: String) -> Error {
    Error::Connect(io::Error::other(format!("proxy settings unreadable: {why}")))
}

fn authority(host: &str, port: u16) -> String {
    match host.contains(':') && !host.starts_with('[') {
        true => format!("[{host}]:{port}"),
        false => format!("{host}:{port}"),
    }
}

/// The first variable set; one not text is an error.
fn variable(names: &[&'static str]) -> Result<Option<(&'static str, String)>> {
    for name in names {
        match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => return Ok(Some((name, value))),
            Ok(_) | Err(std::env::VarError::NotPresent) => {}
            Err(std::env::VarError::NotUnicode(_)) => return Err(unreadable(format!("{name} is not text"))),
        }
    }
    Ok(None)
}

fn env() -> Result<Option<Named>> {
    let Some((from, address)) = variable(&NAMED)? else {
        return Ok(None);
    };
    let skips = variable(&["NO_PROXY", "no_proxy"])?.map(|(_, list)| list).unwrap_or_default();
    let skips = skips.split(',').map(str::trim).filter(|skip| !skip.is_empty()).map(str::to_owned).collect();
    Ok(Some(Named { from, address, skips, system: false }))
}

/// The address without its login, for errors.
fn shown(address: &str) -> String {
    let (scheme, rest) = address.split_once("://").unwrap_or(("", address));
    let host = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    match scheme {
        "" => host.to_owned(),
        _ => format!("{scheme}://{host}"),
    }
}

/// An explicit port must fit; ureq would take the default.
fn port_fits(address: &str) -> bool {
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    let rest = rest.split('/').next().unwrap_or_default();
    let host = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    let tail = host.rfind(']').map_or(host, |end| &host[end + 1..]);
    tail.rsplit_once(':').is_none_or(|(_, port)| port.parse::<u16>().is_ok())
}

/// The login decoded once; a bad setting is an error.
fn build(named: &Named) -> Result<Route> {
    let bad = |why: String| unreadable(format!("{} {:?}: {why}", named.from, shown(&named.address)));
    if !port_fits(&named.address) {
        return Err(bad("the port is out of range".into()));
    }
    let parsed = Proxy::new(&named.address).map_err(|err| bad(err.to_string()))?;
    let decoded = |text: &str| {
        percent_encoding::percent_decode_str(text)
            .decode_utf8()
            .map(|text| text.into_owned())
            .map_err(|err| bad(err.to_string()))
    };
    let login = match parsed.username() {
        Some(user) => Some((decoded(user)?, decoded(parsed.password().unwrap_or_default())?)),
        None => None,
    };
    let mut builder = Proxy::builder(parsed.protocol()).host(parsed.host()).port(parsed.port());
    if let Some(user) = parsed.username() {
        builder = builder.username(user);
    }
    if let Some(password) = parsed.password() {
        builder = builder.password(password);
    }
    let looped = if named.system { &LOOPBACK[..] } else { &[] };
    for skip in named.skips.iter().map(String::as_str).chain(looped.iter().copied()) {
        builder = builder.no_proxy(skip);
    }
    let proxy = builder.build().map_err(|err| bad(err.to_string()))?;
    Ok(Route { proxy, login })
}

#[cfg(windows)]
fn system() -> Result<Option<Named>> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey(path) {
        Ok(key) => key,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(unreadable(format!("Internet Settings: {err}"))),
    };
    registry(Registry {
        enable: key.get_value("ProxyEnable"),
        server: key.get_value("ProxyServer"),
        skips: key.get_value("ProxyOverride"),
        pac: key.get_value("AutoConfigURL"),
    })
}

/// The Internet Settings values, as read.
#[cfg(any(windows, test))]
struct Registry {
    enable: io::Result<u32>,
    server: io::Result<String>,
    skips: io::Result<String>,
    pac: io::Result<String>,
}

/// Missing is unset; any other failure is an error.
#[cfg(any(windows, test))]
fn given<T>(name: &str, value: io::Result<T>) -> Result<Option<T>> {
    match value {
        Ok(value) => Ok(Some(value)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(unreadable(format!("{name}: {err}"))),
    }
}

#[cfg(any(windows, test))]
fn registry(read: Registry) -> Result<Option<Named>> {
    let on = given("ProxyEnable", read.enable)? == Some(1);
    let pac = given("AutoConfigURL", read.pac)?;
    if !on {
        if pac.is_some() {
            log::warn!("net: the system proxy is a PAC file; not followed");
        }
        return Ok(None);
    }
    let server = given("ProxyServer", read.server)?.ok_or_else(|| unreadable("ProxyServer is missing".into()))?;
    let skips = given("ProxyOverride", read.skips)?.unwrap_or_default();
    windows(&server, &skips).map(Some)
}

/// One proxy, or one per scheme: secure web, web, SOCKS.
#[cfg(any(windows, test))]
fn windows(server: &str, skips: &str) -> Result<Named> {
    let address = match server.contains('=') {
        false => format!("http://{}", server.trim()),
        true => {
            let entry = |kind: &str| server.split(';').find_map(|part| part.trim().strip_prefix(kind));
            let chosen = [("https=", "http"), ("http=", "http"), ("socks=", "socks5")]
                .into_iter()
                .find_map(|(kind, scheme)| Some(format!("{scheme}://{}", entry(kind)?)));
            chosen.ok_or_else(|| unreadable("ProxyServer names no proxy".into()))?
        }
    };
    // `<local>` is hosts without a dot; mail hosts have one.
    let skips = skips
        .split(';')
        .map(str::trim)
        .filter(|skip| !skip.is_empty() && *skip != "<local>")
        .map(str::to_owned)
        .collect();
    Ok(Named { from: "ProxyServer", address, skips, system: true })
}

#[cfg(target_os = "macos")]
fn system() -> Result<Option<Named>> {
    let read = std::process::Command::new("/usr/sbin/scutil").arg("--proxy").output();
    match read {
        Ok(out) if out.status.success() => scutil(&String::from_utf8_lossy(&out.stdout)),
        Ok(out) => Err(unreadable(format!("scutil exited {}", out.status))),
        Err(err) => Err(unreadable(format!("scutil: {err}"))),
    }
}

/// Linux sets proxies through the variables only.
#[cfg(not(any(windows, target_os = "macos")))]
fn system() -> Result<Option<Named>> {
    Ok(None)
}

/// The secure web proxy, else SOCKS, else the web proxy.
#[cfg(any(target_os = "macos", test))]
fn scutil(text: &str) -> Result<Option<Named>> {
    let lines: Vec<(&str, &str)> = text
        .lines()
        .filter_map(|line| line.trim().split_once(" : "))
        .collect();
    let value = |key: &str| lines.iter().find(|(name, _)| *name == key).map(|(_, value)| *value);
    let on = |key: &str| value(key) == Some("1");
    let chosen = [("HTTPS", "http"), ("SOCKS", "socks5"), ("HTTP", "http")]
        .into_iter()
        .find(|(prefix, _)| on(&format!("{prefix}Enable")));
    let Some((prefix, scheme)) = chosen else {
        if on("ProxyAutoConfigEnable") {
            log::warn!("net: the system proxy is a PAC file; not followed");
        }
        return Ok(None);
    };
    let host = value(&format!("{prefix}Proxy")).ok_or_else(|| unreadable(format!("{prefix} has no host")))?;
    let port = value(&format!("{prefix}Port"))
        .and_then(|port| port.parse().ok())
        .ok_or_else(|| unreadable(format!("{prefix} has no port")))?;
    let mut skips = Vec::new();
    let mut listing = false;
    for line in text.lines().map(str::trim) {
        match line {
            "ExceptionsList : <array> {" => listing = true,
            "}" => listing = false,
            _ if listing => skips.extend(line.split_once(" : ").map(|(_, skip)| skip.to_owned())),
            _ => {}
        }
    }
    let address = format!("{scheme}://{}", authority(host, port));
    Ok(Some(Named { from: "scutil", address, skips, system: true }))
}

/// A stream to `host:port` through the route's proxy.
pub(crate) async fn tunnel(route: &Route, host: &str, port: u16) -> Result<TcpStream> {
    let proxy = &route.proxy;
    let mut stream = TcpStream::connect((socket_host(proxy), proxy.port())).await.map_err(Error::Connect)?;
    let login = route.login.as_ref().map(|(user, password)| (user.as_str(), password.as_str()));
    let opened = match proxy.protocol() {
        ProxyProtocol::Http => connect(&mut stream, login, host, port).await,
        ProxyProtocol::Socks5 | ProxyProtocol::Socks5h => socks5(&mut stream, login, host, port).await,
        other => Err(io::Error::other(format!("{other:?} proxies are not supported"))),
    };
    opened.map_err(Error::Connect)?;
    Ok(stream)
}

/// Sockets take an IPv6 literal without brackets.
fn socket_host(proxy: &Proxy) -> &str {
    proxy.host().trim_start_matches('[').trim_end_matches(']')
}

/// HTTP CONNECT; byte by byte, leaving the server's bytes unread.
async fn connect(stream: &mut TcpStream, login: Option<(&str, &str)>, host: &str, port: u16) -> io::Result<()> {
    let target = authority(host, port);
    let mut ask = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
    if let Some((user, password)) = login {
        let pair = format!("{user}:{password}");
        ask.push_str(&format!("Proxy-Authorization: Basic {}\r\n", STANDARD.encode(pair)));
    }
    ask.push_str("\r\n");
    stream.write_all(ask.as_bytes()).await?;
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= 8192 {
            return Err(io::Error::other("the proxy's reply has no end"));
        }
        head.push(stream.read_u8().await?);
    }
    let text = String::from_utf8_lossy(&head);
    let status = text.lines().next().unwrap_or_default();
    match status.split(' ').nth(1) {
        Some("200") => Ok(()),
        _ => Err(io::Error::other(format!("the proxy refused: {status}"))),
    }
}

/// RFC 1928, with RFC 1929's password when given.
async fn socks5(stream: &mut TcpStream, login: Option<(&str, &str)>, host: &str, port: u16) -> io::Result<()> {
    let methods: &[u8] = if login.is_some() { &[0, 2] } else { &[0] };
    stream.write_all(&[&[5, methods.len() as u8], methods].concat()).await?;
    let mut chosen = [0u8; 2];
    stream.read_exact(&mut chosen).await?;
    match (chosen, login) {
        ([5, 0], _) => {}
        ([5, 2], Some((user, password))) => {
            let (user, password) = (short(user)?, short(password)?);
            let ask = [&[1, user.len() as u8], user, &[password.len() as u8], password].concat();
            stream.write_all(&ask).await?;
            let mut answer = [0u8; 2];
            stream.read_exact(&mut answer).await?;
            if answer[1] != 0 {
                return Err(io::Error::other("the proxy refused the password"));
            }
        }
        _ => return Err(io::Error::other("the proxy takes no method offered")),
    }
    let name = short(host)?;
    let ask = [&[5, 1, 0, 3, name.len() as u8], name, &port.to_be_bytes()].concat();
    stream.write_all(&ask).await?;
    let mut reply = [0u8; 4];
    stream.read_exact(&mut reply).await?;
    if reply[1] != 0 {
        return Err(io::Error::other(format!("the proxy refused: SOCKS reply {}", reply[1])));
    }
    let bound = match reply[3] {
        1 => 4,
        4 => 16,
        3 => stream.read_u8().await? as usize,
        other => return Err(io::Error::other(format!("a SOCKS address of type {other}"))),
    };
    let mut rest = vec![0u8; bound + 2];
    stream.read_exact(&mut rest).await?;
    Ok(())
}

/// SOCKS fields carry at most 255 bytes.
fn short(text: &str) -> io::Result<&[u8]> {
    match text.len() <= 255 {
        true => Ok(text.as_bytes()),
        false => Err(io::Error::other("a SOCKS field longer than 255 bytes")),
    }
}

#[cfg(test)]
mod tests;
