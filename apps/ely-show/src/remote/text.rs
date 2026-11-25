//! `text set` and `notes get|set` by slide number (#107).

use ely_lok::show::Slide;
use ely_office::remote;

use super::slide::{NUMBER, number};

const SLIDE_BOX: &str = "slide {slide} has no {box} placeholder";
const BOX_NAME: &str = "\"{box}\": must be title or body";
pub(super) const FIND_BREAK: &str = "FIND must be non-empty text within one paragraph";
const BOX_RICH: &str = "slide {slide}: mixed formatting in {box}; edit it in ElyShow";

/// `text set`: file, slide number, box, text.
pub(super) fn set_request(path: &str, slide: &str, box_name: &str, text: &str) -> Result<Vec<String>, String> {
    if !matches!(box_name, "title" | "body") {
        return Err(BOX_NAME.replace("{box}", box_name));
    }
    Ok(vec!["set".into(), remote::absolute(path)?, number(slide)?.to_string(), box_name.into(), text.into()])
}

/// `notes get|set`: sent as `notes.get` or `notes.set`.
pub(super) fn notes_request(words: &[&str]) -> Result<Vec<String>, String> {
    Ok(match *words {
        ["get", path, slide] => vec!["notes.get".into(), remote::absolute(path)?, number(slide)?.to_string()],
        ["set", path, slide, text] => {
            vec!["notes.set".into(), remote::absolute(path)?, number(slide)?.to_string(), text.into()]
        }
        _ => return Err("unknown command".into()),
    })
}

/// Line breaks as the kernel splits paragraphs.
pub(super) fn lines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Slide number N as a zero-based index, if present.
pub(super) fn slide_at<'a>(slides: &'a [Slide], slide: &str) -> Result<(usize, &'a Slide), String> {
    let missing = || NUMBER.replace("{slide}", slide).replace("{count}", &slides.len().to_string());
    let at = slide.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).ok_or_else(missing)?;
    slides.get(at).map(|found| (at, found)).ok_or_else(missing)
}

/// The slide's index when its `box_name` placeholder takes plain text.
pub(super) fn writable(slides: &[Slide], slide: &str, box_name: &str) -> Result<usize, String> {
    let (at, found) = slide_at(slides, slide)?;
    match found.boxes.iter().find(|(name, _)| name == box_name) {
        None => Err(SLIDE_BOX.replace("{slide}", slide).replace("{box}", box_name)),
        Some((_, true)) => Err(BOX_RICH.replace("{slide}", slide).replace("{box}", box_name)),
        Some((_, false)) => Ok(at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck() -> Vec<Slide> {
        let slide = |boxes: &[(&str, bool)]| Slide {
            name: "Slide".into(),
            layout: 1,
            title: String::new(),
            hidden: false,
            notes: "Hi\nthere".into(),
            boxes: boxes.iter().map(|(name, rich)| ((*name).into(), *rich)).collect(),
        };
        vec![slide(&[("title", false), ("body", false), ("notes", false)]), slide(&[("title", true), ("notes", true)])]
    }

    #[test]
    fn requests_check_the_box_and_number() {
        let sent = set_request("a.pptx", "2", "body", "x").expect("a request");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("set", "2", "body"));
        assert_eq!(set_request("a.pptx", "1", "notes", "x"), Err(BOX_NAME.replace("{box}", "notes")));
        assert!(set_request("a.pptx", "0", "title", "x").is_err(), "slides count from 1");
        assert_eq!(notes_request(&["get", "a.pptx", "3"]).expect("get")[0], "notes.get");
        assert_eq!(notes_request(&["set", "a.pptx", "3", ""]).expect("set")[3], "");
        assert!(notes_request(&["set", "a.pptx", "3"]).is_err(), "no text");
    }

    #[test]
    fn boxes_must_be_there_and_of_one_look() {
        let slides = deck();
        assert_eq!(writable(&slides, "1", "body"), Ok(0));
        assert_eq!(writable(&slides, "2", "body"), Err(SLIDE_BOX.replace("{slide}", "2").replace("{box}", "body")));
        assert_eq!(writable(&slides, "2", "notes"), Err(BOX_RICH.replace("{slide}", "2").replace("{box}", "notes")));
        assert_eq!(writable(&slides, "3", "title"), Err(NUMBER.replace("{slide}", "3").replace("{count}", "2")));
        assert_eq!(slide_at(&slides, "1").map(|(_, found)| found.notes.as_str()), Ok("Hi\nthere"));
        assert_eq!(lines("a\r\nb\rc\n"), "a\nb\nc\n");
    }
}
