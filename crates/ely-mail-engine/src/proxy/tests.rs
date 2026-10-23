use ureq::{ProxyProtocol, http::Uri};

use super::{Named, build, scutil, web, windows};

/// `scutil --proxy` with ClashX's settings.
const CLASH: &str = "<dictionary> {
  ExceptionsList : <array> {
    0 : *.local
  }
  FTPPassive : 1
  HTTPEnable : 1
  HTTPPort : 7890
  HTTPProxy : 127.0.0.1
  HTTPSEnable : 1
  HTTPSPort : 7891
  HTTPSProxy : 127.0.0.1
  SOCKSEnable : 1
  SOCKSPort : 7892
  SOCKSProxy : 127.0.0.2
}
";

fn skipped(proxy: &ureq::Proxy, host: &str) -> bool {
    proxy.is_no_proxy(&format!("http://{host}/").parse::<Uri>().unwrap())
}

fn named(address: &str, system: bool) -> Named {
    Named { from: "test", address: address.into(), skips: Vec::new(), system }
}

#[test]
fn the_secure_web_proxy_comes_first_with_its_exceptions() {
    let proxy = build(&scutil(CLASH).unwrap().expect("a proxy")).unwrap().proxy;
    assert_eq!((proxy.protocol(), proxy.host(), proxy.port()), (ProxyProtocol::Http, "127.0.0.1", 7891));
    assert!(skipped(&proxy, "printer.local"));
    assert!(!skipped(&proxy, "imap.gmail.com"));
}

#[test]
fn socks_stands_in_when_no_web_proxy_is_on() {
    let text = CLASH.replace("HTTPEnable : 1", "HTTPEnable : 0").replace("HTTPSEnable : 1", "HTTPSEnable : 0");
    let proxy = build(&scutil(&text).unwrap().expect("a proxy")).unwrap().proxy;
    assert_eq!((proxy.protocol(), proxy.host(), proxy.port()), (ProxyProtocol::Socks5, "127.0.0.2", 7892));
}

#[test]
fn none_on_or_only_a_pac_file_goes_straight() {
    assert!(scutil("<dictionary> {\n  HTTPEnable : 0\n}\n").unwrap().is_none());
    let pac = "<dictionary> {\n  ProxyAutoConfigEnable : 1\n  ProxyAutoConfigURLString : http://x/p.pac\n}\n";
    assert!(scutil(pac).unwrap().is_none());
}

#[test]
fn a_proxy_on_without_a_port_is_an_error() {
    let err = scutil("<dictionary> {\n  HTTPSEnable : 1\n  HTTPSProxy : 127.0.0.1\n}\n").err().expect("an error");
    assert!(err.to_string().contains("HTTPS has no port"), "{err}");
}

#[test]
fn windows_reads_one_proxy_or_one_per_scheme() {
    let proxy = build(&windows("127.0.0.1:7890", "localhost;*.corp.example;<local>").unwrap()).unwrap().proxy;
    assert_eq!((proxy.protocol(), proxy.host(), proxy.port()), (ProxyProtocol::Http, "127.0.0.1", 7890));
    assert!(skipped(&proxy, "mail.corp.example"));
    assert!(!skipped(&proxy, "imap.gmail.com"));
    let each = build(&windows("ftp=f:1;http=h:2;https=s:3;socks=k:4", "").unwrap()).unwrap().proxy;
    assert_eq!((each.protocol(), each.host(), each.port()), (ProxyProtocol::Http, "s", 3));
    let socks = build(&windows("socks=k:4", "").unwrap()).unwrap().proxy;
    assert_eq!((socks.protocol(), socks.host(), socks.port()), (ProxyProtocol::Socks5, "k", 4));
    let err = windows("ftp=ann:secret@f:1", "").err().expect("an error").to_string();
    assert!(err.contains("names no proxy") && !err.contains("secret"), "{err}");
}

#[test]
fn loopback_skips_the_system_proxy_but_not_a_named_one() {
    let system = build(&named("http://127.0.0.1:7890", true)).unwrap().proxy;
    let variable = build(&named("http://127.0.0.1:7890", false)).unwrap().proxy;
    for host in ["127.0.0.1", "localhost", "[::1]"] {
        assert!(skipped(&system, host), "{host}");
        assert!(!skipped(&variable, host), "{host}");
    }
    assert!(!skipped(&system, "imap.gmail.com"));
}

#[test]
fn credentials_are_decoded_and_bad_settings_refused() {
    let route = build(&named("http://ann:pa%3As%40s@127.0.0.1:7890", false)).unwrap();
    assert_eq!(route.login, Some(("ann".into(), "pa:s@s".into())));
    let err = web(route).expect_err("an error").to_string();
    assert!(err.contains("a proxy login with escapes"), "{err}");
    let plain = build(&named("http://ann:pass@127.0.0.1:7890", false)).unwrap();
    assert_eq!(web(plain).unwrap().password(), Some("pass"));
    let err = build(&named("http://[broken", false)).err().expect("an error").to_string();
    assert!(err.contains("proxy settings unreadable: test"), "{err}");
}

#[test]
fn errors_carry_no_login_and_ports_must_fit() {
    let err = build(&named("http://ann:secret@[broken", false)).err().expect("an error").to_string();
    assert!(err.contains("\"http://[broken\"") && !err.contains("secret"), "{err}");
    for address in ["http://127.0.0.1:99999", "http://ann:secret@[::1]:70000", "127.0.0.1:65536"] {
        let err = build(&named(address, false)).err().expect("an error").to_string();
        assert!(err.contains("the port is out of range") && !err.contains("secret"), "{err}");
    }
    assert_eq!(build(&named("http://[::1]:7890", false)).unwrap().proxy.port(), 7890);
}

#[test]
fn an_ipv6_system_proxy_reads() {
    let text = "<dictionary> {\n  HTTPSEnable : 1\n  HTTPSPort : 7890\n  HTTPSProxy : ::1\n}\n";
    let proxy = build(&scutil(text).unwrap().expect("a proxy")).unwrap().proxy;
    assert_eq!((proxy.host().trim_matches(['[', ']']), proxy.port()), ("::1", 7890));
}

fn missing<T>() -> std::io::Result<T> {
    Err(std::io::ErrorKind::NotFound.into())
}

#[test]
fn registry_values_missing_are_unset_and_unreadable_ones_errors() {
    let read = |enable, skips| super::Registry { enable, server: Ok("127.0.0.1:7890".into()), skips, pac: missing() };
    assert!(super::registry(read(missing(), missing())).unwrap().is_none());
    assert!(super::registry(read(Ok(0), missing())).unwrap().is_none());
    let named = super::registry(read(Ok(1), Ok("*.corp.example".into()))).unwrap().expect("a proxy");
    assert!(skipped(&build(&named).unwrap().proxy, "mail.corp.example"));
    let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    let err = super::registry(read(Err(denied), missing())).err().expect("an error").to_string();
    assert!(err.contains("ProxyEnable"), "{err}");
    let mistyped = std::io::Error::from(std::io::ErrorKind::InvalidData);
    let err = super::registry(read(Ok(1), Err(mistyped))).err().expect("an error").to_string();
    assert!(err.contains("ProxyOverride"), "{err}");
    let gone = super::Registry { enable: Ok(1), server: missing(), skips: missing(), pac: missing() };
    assert!(super::registry(gone).err().expect("an error").to_string().contains("ProxyServer is missing"));
}

#[test]
fn an_ipv6_proxy_is_dialled_without_brackets() {
    let proxy = build(&named("http://[::1]:7890", false)).unwrap().proxy;
    assert_eq!(super::socket_host(&proxy), "::1");
    let named_host = build(&named("http://proxy.example:7890", false)).unwrap().proxy;
    assert_eq!(super::socket_host(&named_host), "proxy.example");
}
