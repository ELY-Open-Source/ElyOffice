//! `mailto:` links (RFC 6068) as a message to write.

use mail_parser::MessageParser;

/// What a mail link asks to write.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Asked {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    /// Plain text, lines ending in `\n`.
    pub body: String,
}

pub fn parse(link: &str) -> Result<Asked, String> {
    let rest = link
        .get(..7)
        .filter(|scheme| scheme.eq_ignore_ascii_case("mailto:"))
        .map(|_| &link[7..])
        .ok_or_else(|| "not a mailto link".to_owned())?;
    let (to, fields) = rest.split_once('?').unwrap_or((rest, ""));
    let mut asked = Asked::default();
    add(&mut asked.to, &decoded(to)?)?;
    for field in fields.split('&').filter(|field| !field.is_empty()) {
        let (name, value) = field.split_once('=').ok_or_else(|| format!("the field {field} has no value"))?;
        let value = decoded(value)?;
        match decoded(name)?.to_ascii_lowercase().as_str() {
            "to" => add(&mut asked.to, &value)?,
            "cc" => add(&mut asked.cc, &value)?,
            "bcc" => add(&mut asked.bcc, &value)?,
            "subject" => asked.subject = subject(&value)?,
            "body" => asked.body = value.replace("\r\n", "\n"),
            other => log::info!("mail: a mail link's {other} field is left out"),
        }
    }
    Ok(asked)
}

/// Splits at commas outside quotes; domains go to ASCII.
fn add(list: &mut Vec<String>, addresses: &str) -> Result<(), String> {
    let (mut quoted, mut escaped, mut start) = (false, false, 0);
    let mut parts = Vec::new();
    for (at, char) in addresses.char_indices() {
        match char {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            ',' if !quoted => {
                parts.push(&addresses[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    if quoted {
        return Err(format!("an open quote in {addresses}"));
    }
    parts.push(&addresses[start..]);
    for address in parts.into_iter().map(str::trim).filter(|address| !address.is_empty()) {
        list.push(ascii_domain(address)?);
    }
    Ok(())
}

/// The domain after the last `@` in IDNA form.
fn ascii_domain(address: &str) -> Result<String, String> {
    let Some((local, domain)) = address.rsplit_once('@') else {
        return Ok(address.to_owned());
    };
    let domain = idna::domain_to_ascii(domain).map_err(|_| format!("{domain} is not a domain"))?;
    Ok(format!("{local}@{domain}"))
}

/// Encoded words (RFC 2047) read as text.
fn subject(value: &str) -> Result<String, String> {
    let one_line = |text: &str| match text.contains(['\r', '\n']) {
        true => Err("a subject holds no line breaks".to_owned()),
        false => Ok(()),
    };
    one_line(value)?;
    let header = format!("Subject: {value}\r\n\r\n");
    let parsed = MessageParser::default().parse_headers(header.as_bytes());
    let read = parsed.and_then(|message| message.subject().map(str::to_owned)).unwrap_or_default();
    // Encoded words can carry them too.
    one_line(&read)?;
    Ok(read)
}

/// Percent escapes as UTF-8; `+` stays a plus.
fn decoded(text: &str) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut raw = text.bytes();
    while let Some(byte) = raw.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let value = match [raw.next(), raw.next()] {
            [Some(high), Some(low)] if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() => {
                std::str::from_utf8(&[high, low]).ok().and_then(|hex| u8::from_str_radix(hex, 16).ok())
            }
            _ => None,
        };
        bytes.push(value.ok_or_else(|| format!("a broken escape in {text}"))?);
    }
    String::from_utf8(bytes).map_err(|_| format!("{text} is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::{Asked, parse};

    #[test]
    fn a_link_fills_every_field_it_names() {
        let asked = parse("mailto:a@b.c,%20d@e.f?Subject=Hi%20there&cc=g@h.i&bcc=j@k.l&to=m@n.o&body=one%0D%0Atwo%E4%BD%A0+3")
            .expect("a link");
        assert_eq!(
            asked,
            Asked {
                to: vec!["a@b.c".into(), "d@e.f".into(), "m@n.o".into()],
                cc: vec!["g@h.i".into()],
                bcc: vec!["j@k.l".into()],
                subject: "Hi there".into(),
                body: "one\ntwo你+3".into(),
            }
        );
    }

    #[test]
    fn the_rfc_examples_read_as_meant() {
        let quoted = parse("mailto:%22last%2Cfirst%22@example.org,b@c.d?cc=%22x%5C%22%2Cy%22@e.f").expect("quoted");
        assert_eq!(quoted.to, ["\"last,first\"@example.org", "b@c.d"]);
        assert_eq!(quoted.cc, ["\"x\\\",y\"@e.f"]);
        let natto = parse("mailto:user@%E7%B4%8D%E8%B1%86.example.org?subject=Test&body=NATTO").expect("natto");
        assert_eq!(natto.to, ["user@xn--99zt52a.example.org"]);
        let cafe = parse("mailto:user@example.org?subject=%3D%3Futf-8%3FQ%3Fcaf%3DC3%3DA9%3F%3D").expect("café");
        assert_eq!(cafe.subject, "café");
    }

    #[test]
    fn bare_and_unknown_fields_are_kept_apart() {
        assert_eq!(parse("MAILTO:a@b.c").expect("bare").to, ["a@b.c"]);
        let asked = parse("mailto:?subject=x&in-reply-to=%3Cy%3E").expect("no address");
        assert!(asked.to.is_empty());
        assert_eq!(asked.subject, "x");
    }

    #[test]
    fn broken_links_are_refused() {
        for link in [
            "mailto:a@b.c?subject=%zz",
            "mailto:a%4",
            "mailto:a@b.c?subject=%FF",
            "mailto:a@b.c?body",
            "mailto:a@b.c?body=%+A",
            "mailto:a@b.c?body=%-1",
            "mailto:%22a@b.c",
            "mailto:a@b.c?subject=x%0Ay",
            "mailto:a@b.c?subject=%3D%3FUTF-8%3FQ%3Fa%3D0D%3D0Ab%3F%3D",
            "a@b.c",
        ] {
            assert!(parse(link).is_err(), "{link}");
        }
    }
}
