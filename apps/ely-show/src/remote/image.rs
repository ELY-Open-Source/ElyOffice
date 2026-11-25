//! `image insert FILE N IMAGE [--width SIZE]` (#107).

use ely_office::remote::{self, length};

use super::slide::number;

const MISSING: &str = "{image}: image file not found";
pub(super) const READ: &str = "{image}: cannot read as an image; nothing was inserted";
const WIDTH: &str = "invalid --width \"{size}\" (use a number with cm, mm, in or pt, from 1pt to 100cm)";

/// Sent: verb, file, slide, URL, twips, IMAGE as typed.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    let (path, slide, image, width) = match *words {
        ["insert", path, slide, image] => (path, slide, image, None),
        ["insert", path, slide, image, "--width", size] => (path, slide, image, Some(size)),
        _ => return Err("unknown command".into()),
    };
    let slide = number(slide)?.to_string();
    let width = match width {
        Some(size) => length::twips(size).map_err(|_| WIDTH.replace("{size}", size))?.to_string(),
        None => String::new(),
    };
    let picture = std::path::absolute(image).map_err(|error| format!("{image}: {error}"))?;
    if !picture.is_file() {
        return Err(MISSING.replace("{image}", image));
    }
    let url = ely_lok::file_url(&picture);
    Ok(vec!["image.insert".into(), remote::absolute(path)?, slide, url, width, image.to_owned()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("dir");
        let png = dir.path().join("a.png");
        std::fs::write(&png, b"x").expect("a file");
        let named = png.display().to_string();
        (dir, named)
    }

    #[test]
    fn the_picture_goes_as_a_url_with_its_slide_and_width() {
        let (_dir, named) = picture();
        let sent = request(&["insert", "a.pptx", "2", &named]).expect("fine");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[4].as_str()), ("image.insert", "2", ""));
        assert!(std::path::Path::new(&sent[1]).is_absolute(), "{sent:?}");
        assert!(sent[3].starts_with("file://"), "{sent:?}");
        assert_eq!(sent[5], named);
        assert_eq!(request(&["insert", "a.pptx", "1", &named, "--width", "8cm"]).expect("a width")[4], "4535");
        assert_eq!(request(&["insert", "a.pptx", "1", &named, "--width", "100cm"]).expect("the top")[4], "56693");
        assert_eq!(request(&["insert", "a.pptx", "1", &named, "--width", "1pt"]).expect("the bottom")[4], "20");
    }

    #[test]
    fn a_missing_picture_bad_slide_width_or_flags_are_refused() {
        let (dir, named) = picture();
        let gone = dir.path().join("gone.png").display().to_string();
        assert_eq!(request(&["insert", "a.pptx", "1", &gone]), Err(MISSING.replace("{image}", &gone)));
        let folder = dir.path().display().to_string();
        assert_eq!(request(&["insert", "a.pptx", "1", &folder]), Err(MISSING.replace("{image}", &folder)), "a folder");
        for size in ["8", "8px", "0.5pt", "100.01cm", "cm", "-1cm"] {
            assert_eq!(request(&["insert", "a.pptx", "1", &named, "--width", size]), Err(WIDTH.replace("{size}", size)), "{size}");
        }
        for slide in ["0", "x", "-1"] {
            assert_eq!(request(&["insert", "a.pptx", slide, &named]), Err(number(slide).expect_err("not a number")), "{slide}");
        }
        for flags in [&["--width"][..], &["--width", "1cm", "--width", "2cm"], &["--after", "x"]] {
            let words = [&["insert", "a.pptx", "1", &named][..], flags].concat();
            assert!(request(&words).is_err(), "{flags:?}");
        }
        assert!(request(&["insert", "a.pptx", &named]).is_err(), "no slide");
    }
}
