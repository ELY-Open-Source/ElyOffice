//! Which link addresses may leave for the system opener.

/// Addresses a click may open outside: web and mail.
pub fn outward(uri: &str) -> bool {
    // Schemes ignore case.
    let has = |scheme: &str| uri.get(..scheme.len()).is_some_and(|head| head.eq_ignore_ascii_case(scheme));
    if has("mailto:") {
        return !attaches(&uri["mailto:".len()..]);
    }
    has("https://") || has("http://")
}

/// Whether mail headers ask a client to attach a file.
fn attaches(mail: &str) -> bool {
    let Some((_, headers)) = mail.split_once('?') else {
        return false;
    };
    headers.split('&').any(|header| {
        let name = header.split_once('=').map_or(header, |(name, _)| name).trim().to_ascii_lowercase();
        // An encoded name may hide "attach".
        name.starts_with("attach") || name.contains('%')
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn web_and_mail_go_out_in_any_case() {
        for uri in [
            "https://a.example/",
            "HTTPS://a.example/",
            "Http://a.example/",
            "MAILTO:x@a.example",
            "mailto:x@a.example?subject=attach%20it&body=see%20attached",
        ] {
            assert!(super::outward(uri), "{uri}");
        }
        for uri in ["ftp://a.example/", "file:///etc/passwd", "javascript:alert(1)", "httpsx://a", "ht", "\u{e9}https://"] {
            assert!(!super::outward(uri), "{uri}");
        }
    }

    #[test]
    fn mail_that_attaches_a_file_stays_in() {
        for uri in [
            "mailto:x@a.example?attach=/etc/passwd",
            "mailto:x@a.example?subject=hi&Attachment=file:///home/u/.ssh/id_rsa",
            "MAILTO:x@a.example?body=x& ATTACH =~/.ssh/id_rsa",
            "mailto:x@a.example?%61ttach=/etc/passwd",
        ] {
            assert!(!super::outward(uri), "{uri}");
        }
    }
}
