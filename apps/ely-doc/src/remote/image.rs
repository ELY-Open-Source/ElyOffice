//! `image insert FILE IMAGE [--after FIND] [--width SIZE]` (#107).

use ely_office::remote::{self, length};

use super::text::FIND_PARAGRAPH;

pub(super) const MISSING: &str = "{image}: image file not found";
pub(super) const READ: &str = "{image}: cannot read as an image; nothing was inserted";
const WIDTH: &str = "invalid --width \"{size}\" (use a number with cm, mm, in or pt, from 1pt to 100cm)";

/// Sent: verb, file, URL, FIND, width, IMAGE as typed.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    let ["insert", path, image, flags @ ..] = words else {
        return Err("unknown command".into());
    };
    let (mut after, mut width) = (None, None);
    let mut flags = flags.iter();
    while let Some(flag) = flags.next() {
        let slot = match *flag {
            "--after" if after.is_none() => &mut after,
            "--width" if width.is_none() => &mut width,
            _ => return Err("unknown command".into()),
        };
        *slot = Some(*flags.next().ok_or("unknown command")?);
    }
    if after.is_some_and(|find| find.is_empty() || find.contains(['\n', '\r'])) {
        return Err(FIND_PARAGRAPH.into());
    }
    let width = match width {
        Some(size) => length::twips(size).map_err(|_| WIDTH.replace("{size}", size))?.to_string(),
        None => String::new(),
    };
    let picture = std::path::absolute(image).map_err(|error| format!("{image}: {error}"))?;
    if !picture.is_file() {
        return Err(MISSING.replace("{image}", image));
    }
    let url = ely_lok::file_url(&picture);
    let after = after.unwrap_or_default().to_owned();
    Ok(vec!["image.insert".into(), remote::absolute(path)?, url, after, width, (*image).to_owned()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_goes_as_a_url_with_its_width_in_twips() {
        let dir = tempfile::tempdir().expect("dir");
        let png = dir.path().join("a.png");
        std::fs::write(&png, b"x").expect("a file");
        let named = png.display().to_string();
        let sent = request(&["insert", "a.docx", &named]).expect("fine");
        assert_eq!((sent[0].as_str(), sent[5].as_str()), ("image.insert", named.as_str()));
        assert!(sent[2].starts_with("file://"), "{sent:?}");
        assert_eq!((sent[3].as_str(), sent[4].as_str()), ("", ""));
        let sent = request(&["insert", "a.docx", &named, "--width", "8cm", "--after", "第一章"]).expect("either order");
        assert_eq!((sent[3].as_str(), sent[4].as_str()), ("第一章", "4535"));
        assert_eq!(request(&["insert", "a.docx", &named, "--width", "100cm"]).expect("the top")[4], "56693");
        assert_eq!(request(&["insert", "a.docx", &named, "--width", "1pt"]).expect("the bottom")[4], "20");
    }

    #[test]
    fn a_missing_picture_bad_width_or_flags_are_refused() {
        let dir = tempfile::tempdir().expect("dir");
        let png = dir.path().join("a.png");
        std::fs::write(&png, b"x").expect("a file");
        let named = png.display().to_string();
        let gone = dir.path().join("gone.png").display().to_string();
        assert_eq!(request(&["insert", "a.docx", &gone]), Err(MISSING.replace("{image}", &gone)));
        let dir_named = dir.path().display().to_string();
        assert_eq!(request(&["insert", "a.docx", &dir_named]), Err(MISSING.replace("{image}", &dir_named)), "a folder");
        for size in ["8", "8px", "0.5pt", "100.01cm", "cm", "-1cm"] {
            assert_eq!(request(&["insert", "a.docx", &named, "--width", size]), Err(WIDTH.replace("{size}", size)), "{size}");
        }
        assert_eq!(request(&["insert", "a.docx", &named, "--after", ""]), Err(FIND_PARAGRAPH.into()));
        for flags in [&["--after"][..], &["--width"], &["--width", "1cm", "--width", "2cm"], &["--header"]] {
            let words = [&["insert", "a.docx", &named][..], flags].concat();
            assert!(request(&words).is_err(), "{flags:?}");
        }
    }
}
