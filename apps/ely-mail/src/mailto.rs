//! `mailto:` links (RFC 6068) as a message to write.

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
    add(&mut asked.to, &decoded(to)?);
    for field in fields.split('&').filter(|field| !field.is_empty()) {
        let (name, value) = field.split_once('=').ok_or_else(|| format!("the field {field} has no value"))?;
        let value = decoded(value)?;
        match decoded(name)?.to_ascii_lowercase().as_str() {
            "to" => add(&mut asked.to, &value),
            "cc" => add(&mut asked.cc, &value),
            "bcc" => add(&mut asked.bcc, &value),
            "subject" => asked.subject = value,
            "body" => asked.body = value.replace("\r\n", "\n"),
            other => log::info!("mail: a mail link's {other} field is left out"),
        }
    }
    Ok(asked)
}

fn add(list: &mut Vec<String>, addresses: &str) {
    let named = addresses.split(',').map(str::trim).filter(|address| !address.is_empty());
    list.extend(named.map(str::to_owned));
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
        let hex = [raw.next(), raw.next()];
        let value = match hex {
            [Some(high), Some(low)] => std::str::from_utf8(&[high, low])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok()),
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
    fn bare_and_unknown_fields_are_kept_apart() {
        assert_eq!(parse("MAILTO:a@b.c").expect("bare").to, ["a@b.c"]);
        let asked = parse("mailto:?subject=x&in-reply-to=%3Cy%3E").expect("no address");
        assert!(asked.to.is_empty());
        assert_eq!(asked.subject, "x");
    }

    #[test]
    fn broken_links_are_refused() {
        for link in ["mailto:a@b.c?subject=%zz", "mailto:a%4", "mailto:a@b.c?subject=%FF", "mailto:a@b.c?body", "a@b.c"] {
            assert!(parse(link).is_err(), "{link}");
        }
    }
}
