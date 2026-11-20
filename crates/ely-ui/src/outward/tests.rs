use super::{MALFORMED, REFUSED, outward};

#[test]
fn web_and_mail_go_out_in_any_case() {
    for uri in [
        "https://a.example/",
        "HTTPS://a.example/",
        "Http://a.example/",
        "MAILTO:x@a.example",
        "mailto:x@a.example?subject=attach%20it&body=see%20attached",
    ] {
        assert!(outward(uri).is_ok(), "{uri}");
    }
    for uri in [
        "ftp://a.example/",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "vbscript:msgbox(1)",
        "data:text/html,x",
        "smb://host/share",
        "\\\\host\\share",
        "ms-msdt:/id x",
        "httpsx://a",
        "ht",
        "\u{e9}https://",
    ] {
        assert_eq!(outward(uri), Err(REFUSED), "{uri}");
    }
}

/// Raw addresses as PowerPoint keeps them, and what goes out.
const RAW: [(&str, &str); 9] = [
    ("https://a.example/x y\"z'w", "https://a.example/x%20y%22z'w"),
    ("https://a.example/p?q=a b\"c<d>", "https://a.example/p?q=a%20b%22c%3Cd%3E"),
    ("https://a.example/a|b^c{d}`e[f]#g#h", "https://a.example/a%7Cb%5Ec%7Bd%7D%60e%5Bf%5D#g%23h"),
    ("https://a.example/\u{4e2d}", "https://a.example/%E4%B8%AD"),
    ("https://b\u{fc}cher.example/", "https://xn--bcher-kva.example/"),
    ("mailto:x@a.example?subject=a b", "mailto:x@a.example?subject=a%20b"),
    // Credentials: a stray `%` is encoded there too.
    ("https://u%ser:pw@a.example/", "https://u%25ser:pw@a.example/"),
    ("https://u%41:p%zz w@[::1]:8080/x", "https://u%41:p%25zz%20w@[::1]:8080/x"),
    ("http://u:100%@a.example/", "http://u:100%25@a.example/"),
];

#[test]
fn a_raw_pptx_address_goes_out_encoded() {
    for (raw, sent) in RAW {
        assert_eq!(outward(raw).as_deref(), Ok(sent), "{raw}");
    }
}

#[test]
fn an_encoded_address_goes_out_as_it_is() {
    for uri in [
        "https://a.example/x%20y%22z'w",
        "https://a.example/p?q=%E4%B8%AD&r=%2F#top",
        "https://u%41:p%20w@[::1]:8080/x",
        "mailto:x@a.example?subject=attach%20it",
    ] {
        assert_eq!(outward(uri).as_deref(), Ok(uri), "{uri}");
    }
    // A `%` that starts no escape is encoded.
    assert_eq!(outward("https://a.example/100%").as_deref(), Ok("https://a.example/100%25"));
    assert_eq!(outward("https://a.example/%zz").as_deref(), Ok("https://a.example/%25zz"));
}

#[test]
fn an_address_that_does_not_parse_says_so() {
    for uri in ["https://a b/", "http://", "https://[::1/", "https://a.example:99999/"] {
        assert_eq!(outward(uri), Err(MALFORMED), "{uri}");
    }
}

#[test]
fn mail_that_attaches_a_file_stays_in() {
    for uri in [
        "mailto:x@a.example?attach=/etc/passwd",
        "mailto:x@a.example?subject=hi&Attachment=file:///home/u/.ssh/id_rsa",
        "MAILTO:x@a.example?body=x& ATTACH =~/.ssh/id_rsa",
        "mailto:x@a.example?%61ttach=/etc/passwd",
        "mailto:x@a.example?subject=a b&attach=/etc/passwd",
    ] {
        assert_eq!(outward(uri), Err(REFUSED), "{uri}");
    }
}

/// NSURL's strict parse takes what goes out, not the raw.
#[cfg(target_os = "macos")]
#[test]
fn every_address_sent_makes_an_nsurl() {
    use objc2_foundation::{NSString, NSURL};
    let strict = |uri: &str| NSURL::URLWithString_encodingInvalidCharacters(&NSString::from_str(uri), false).is_some();
    for (raw, sent) in RAW {
        assert_eq!(outward(raw).as_deref(), Ok(sent), "{raw}");
        assert!(strict(sent), "NSURL takes {sent}");
        assert!(NSURL::URLWithString(&NSString::from_str(sent)).is_some(), "the opener's call takes {sent}");
    }
    assert!(!strict(RAW[0].0), "raw, NSURL refuses it");
}
