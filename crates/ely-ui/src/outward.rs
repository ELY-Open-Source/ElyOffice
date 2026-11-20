//! Which link addresses may leave for the system opener.

/// Shown for a link of a kind Ely never opens.
const REFUSED: &str = "office.link.refused";
/// Shown for a web or mail address that won't parse.
const MALFORMED: &str = "office.link.malformed";

/// Web or mail, encoded for every opener; else the notice.
pub fn outward(uri: &str) -> Result<String, &'static str> {
    // Schemes ignore case.
    let has = |scheme: &str| uri.get(..scheme.len()).is_some_and(|head| head.eq_ignore_ascii_case(scheme));
    let mail = has("mailto:");
    if !mail && !has("https://") && !has("http://") {
        return Err(REFUSED);
    }
    let parsed = url::Url::parse(uri).map_err(|error| {
        log::info!("links: a web or mail address does not parse: {error}");
        MALFORMED
    })?;
    // NSURL takes only RFC 3986; WHATWG leaves stray `%`.
    let text = parsed.as_str();
    let at = |at| parsed[..at].len();
    let (user, host, path) = (at(url::Position::BeforeUsername), at(url::Position::BeforeHost), at(url::Position::BeforePath));
    // The host and port stay: IPv6 keeps its brackets.
    let address = format!("{}{}{}{}", &text[..user], strict(&text[user..host]), &text[host..path], strict(&text[path..]));
    if mail && attaches(&address["mailto:".len()..]) {
        return Err(REFUSED);
    }
    Ok(address)
}

/// Encodes what RFC 3986 bars in userinfo, path, query, fragment.
fn strict(rest: &str) -> String {
    let bytes = rest.as_bytes();
    let mut out = String::with_capacity(rest.len());
    let mut fragment = false;
    for (at, &byte) in bytes.iter().enumerate() {
        let keep = match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => true,
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => true,
            b':' | b'@' | b'/' | b'?' => true,
            // Only the first `#` starts the fragment.
            b'#' => !std::mem::replace(&mut fragment, true),
            // A valid escape stays; a stray `%` is encoded.
            b'%' => bytes.get(at + 1..at + 3).is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit)),
            _ => false,
        };
        if keep {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
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
mod tests;
