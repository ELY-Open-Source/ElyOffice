//! `format FILE FIND PROPERTY=VALUE ...`: every match formatted (#107).

use ely_office::remote::{self, pairs};

use super::text::FIND_PARAGRAPH;

/// No match in the body.
pub(super) const NOTHING: &str = "{file}: no match for \"{find}\"";

/// The kernel's fields, in its order.
const FIELDS: [&str; 7] = ["bold", "italic", "underline", "font", "size", "color", "style"];

/// The words sent: verb, file, FIND, the kernel's fields.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    let [path, find, given @ ..] = words else {
        return Err("unknown command".into());
    };
    if find.is_empty() || find.contains(['\n', '\r']) {
        return Err(FIND_PARAGRAPH.into());
    }
    Ok(vec!["format".into(), remote::absolute(path)?, (*find).into(), fields(given)?])
}

/// The paragraph style the fields name; empty for none.
pub(super) fn style(fields: &str) -> &str {
    fields.rsplit('\t').next().unwrap_or_default()
}

/// `given` as seven tab-separated fields; empty leaves one alone.
fn fields(given: &[&str]) -> Result<String, String> {
    let given = pairs::parse(given, |key| FIELDS.contains(&key))?;
    let mut out = Vec::with_capacity(FIELDS.len());
    for key in FIELDS {
        let Some(&value) = given.get(key) else {
            out.push(String::new());
            continue;
        };
        out.push(match key {
            "bold" | "italic" | "underline" => pairs::switch(key, value).map(|on| if on { "on" } else { "off" }.into())?,
            "size" => pairs::twips(value)?.to_string(),
            "color" => pairs::rgb(key, value)?.map_or("none".into(), |rgb| format!("{rgb:06X}")),
            // `font` and `style` as written; the kernel knows styles.
            _ => value.into(),
        });
    }
    Ok(out.join("\t"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairs_become_the_kernels_seven_fields() {
        let sent = request(&["a.docx", "第一章", "bold=on", "color=#B00020", "style=Heading 1"]).expect("fine");
        assert_eq!((sent[0].as_str(), sent[2].as_str()), ("format", "第一章"));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert_eq!(sent[3], "on\t\t\t\t\tB00020\tHeading 1");
        assert_eq!(style(&sent[3]), "Heading 1");
        let all = fields(&["italic=off", "underline=on", "font=Noto Serif", "size=10.5", "color=none"]).expect("fine");
        assert_eq!(all, "\toff\ton\tNoto Serif\t210\tnone\t");
        assert_eq!(style(&all), "");
    }

    #[test]
    fn a_bad_find_or_pair_is_refused() {
        assert_eq!(request(&["a.docx", "", "bold=on"]), Err(FIND_PARAGRAPH.into()));
        assert_eq!(request(&["a.docx", "a\nb", "bold=on"]), Err(FIND_PARAGRAPH.into()));
        assert_eq!(request(&["a.docx", "x"]), Err("no format properties given (use PROPERTY=VALUE)".into()));
        assert_eq!(request(&["a.docx", "x", "wrap=on"]), Err("wrap: unknown format property".into()));
        assert_eq!(request(&["a.docx", "x", "size=0"]), Err("size: invalid value \"0\" (must be a number from 1 to 999)".into()));
        assert_eq!(request(&["a.docx", "x", "font=a\tb"]), Err("font=a\tb: expected PROPERTY=VALUE".into()));
        assert!(request(&["a.docx"]).is_err(), "no FIND");
    }
}
