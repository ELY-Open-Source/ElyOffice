//! `slide list|add|duplicate|delete|move` (#107).

use ely_lok::show::Slide;
use ely_office::remote;

const NUMBER: &str = "slide {slide} does not exist (must be 1 to {count})";
const POSITION: &str = "position {position} is out of range (must be 1 to {last})";
const LAYOUT: &str = "unknown layout \"{layout}\" (run ely-show --help for layout names)";
pub(super) const ONLY: &str = "cannot delete the only slide";
const INTEGER: &str = "\"{value}\": must be a whole number (1 or more)";

/// Command-line names, in `ely_lok::show::LAYOUTS` order.
const NAMES: [&str; 12] = [
    "blank",
    "title",
    "title-content",
    "title-2-content",
    "title-only",
    "centered-text",
    "title-content-2-content",
    "title-2-content-content",
    "title-content-over-content",
    "title-2-content-over-content",
    "title-4-content",
    "title-6-content",
];

/// A whole number of 1 or more.
fn number(value: &str) -> Result<usize, String> {
    value.parse().ok().filter(|n| *n >= 1).ok_or_else(|| INTEGER.replace("{value}", value))
}

/// The layout id a name stands for.
fn layout(name: &str) -> Result<u32, String> {
    let at = NAMES.iter().position(|known| *known == name).ok_or_else(|| LAYOUT.replace("{layout}", name))?;
    Ok(ely_lok::show::LAYOUTS[at])
}

/// Sent: `slide.VERB`, file, then numbers counted from 1.
pub(super) fn request(words: &[&str]) -> Result<Vec<String>, String> {
    let sent = |verb: &str, path: &str, rest: &[String]| -> Result<Vec<String>, String> {
        Ok([vec![format!("slide.{verb}"), remote::absolute(path)?], rest.to_vec()].concat())
    };
    match *words {
        ["list", path] => sent("list", path, &[]),
        ["add", path, ref rest @ ..] => {
            let (mut position, mut named) = (None, None);
            let mut rest = rest.iter();
            while let Some(word) = rest.next() {
                match *word {
                    "--layout" if named.is_none() => named = Some(*rest.next().ok_or("unknown command")?),
                    _ if position.is_none() && !word.starts_with("--") => position = Some(number(word)?),
                    _ => return Err("unknown command".into()),
                }
            }
            let id = layout(named.unwrap_or("title-content"))?;
            sent("add", path, &[position.map_or_else(String::new, |n| n.to_string()), id.to_string()])
        }
        [verb @ ("duplicate" | "delete"), path, slide] => sent(verb, path, &[number(slide)?.to_string()]),
        ["move", path, slide, to] => sent("move", path, &[number(slide)?.to_string(), number(to)?.to_string()]),
        _ => Err("unknown command".into()),
    }
}

/// `slide list`'s answer: one JSON object per slide.
pub(super) fn listed(slides: &[Slide]) -> String {
    let named = |id: u32| ely_lok::show::LAYOUTS.iter().position(|known| *known == id).map_or("other", |at| NAMES[at]);
    let items: Vec<_> = (slides.iter().enumerate())
        .map(|(at, slide)| {
            serde_json::json!({
                "number": at + 1,
                "name": slide.name,
                "layout": named(slide.layout),
                "title": slide.title,
                "hidden": slide.hidden,
            })
        })
        .collect();
    serde_json::Value::Array(items).to_string()
}

/// The kernel's verb and 0-based numbers, or why not.
pub(super) fn checked(verb: &str, numbers: &[String], count: usize) -> Result<[String; 3], String> {
    let parse = |value: &String| value.parse::<usize>().map_err(|_| INTEGER.replace("{value}", value));
    let slide = |value: &String| {
        let n = parse(value)?;
        (1..=count).contains(&n).then_some(n - 1).ok_or_else(|| NUMBER.replace("{slide}", value).replace("{count}", &count.to_string()))
    };
    let place = |value: &String, last: usize| {
        let n = parse(value)?;
        (1..=last).contains(&n).then_some(n - 1).ok_or_else(|| POSITION.replace("{position}", value).replace("{last}", &last.to_string()))
    };
    let (at, value) = match (verb, numbers) {
        ("add", [position, id]) if position.is_empty() => (count, id.clone()),
        ("add", [position, id]) => (place(position, count + 1)?, id.clone()),
        ("duplicate", [n]) => (slide(n)?, "-1".to_owned()),
        ("delete", [_]) if count < 2 => return Err(ONLY.into()),
        ("delete", [n]) => (slide(n)?, "-1".to_owned()),
        ("move", [n, to]) => (slide(n)?, place(to, count)?.to_string()),
        _ => return Err("unknown command".into()),
    };
    Ok([verb.to_owned(), at.to_string(), value])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_follow_the_kernel_layouts() {
        assert_eq!(layout("blank"), Ok(20));
        assert_eq!(layout("title-6-content"), Ok(34));
        assert_eq!(layout("Title"), Err(LAYOUT.replace("{layout}", "Title")));
        let slide = |layout| Slide { name: "Slide 1".into(), layout, title: "Hi".into(), hidden: false };
        let json: serde_json::Value = serde_json::from_str(&listed(&[slide(1), slide(27)])).expect("JSON");
        assert_eq!(json[0]["layout"], "title-content");
        assert_eq!((json[1]["number"].as_u64(), json[1]["layout"].as_str()), (Some(2), Some("other")));
    }

    #[test]
    fn requests_carry_numbers_and_the_layout_id() {
        let sent = request(&["add", "a.pptx"]).expect("bare add");
        assert_eq!((sent[0].as_str(), sent[2].as_str(), sent[3].as_str()), ("slide.add", "", "1"));
        assert_eq!(request(&["add", "a.pptx", "--layout", "blank", "2"]).expect("either order")[2..], ["2", "20"]);
        assert_eq!(request(&["move", "a.pptx", "3", "1"]).expect("move")[2..], ["3", "1"]);
        assert_eq!(request(&["delete", "a.pptx", "0"]), Err(INTEGER.replace("{value}", "0")));
        assert_eq!(request(&["duplicate", "a.pptx", "x"]), Err(INTEGER.replace("{value}", "x")));
        assert_eq!(request(&["add", "a.pptx", "--layout", "nope"]), Err(LAYOUT.replace("{layout}", "nope")));
        for words in [&["add", "a.pptx", "1", "2"][..], &["add", "a.pptx", "--layout"], &["move", "a.pptx", "1"], &["list"]] {
            assert!(request(words).is_err(), "{words:?}");
        }
    }

    #[test]
    fn numbers_are_checked_against_the_deck() {
        let owned = |values: &[&str]| values.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
        assert_eq!(checked("add", &owned(&["", "1"]), 3), Ok(["add".into(), "3".into(), "1".into()]));
        assert_eq!(checked("add", &owned(&["4", "1"]), 3).map(|s| s[1].clone()), Ok("3".into()), "after the last");
        assert_eq!(checked("add", &owned(&["5", "1"]), 3), Err(POSITION.replace("{position}", "5").replace("{last}", "4")));
        assert_eq!(checked("delete", &owned(&["1"]), 1), Err(ONLY.into()));
        assert_eq!(checked("duplicate", &owned(&["4"]), 3), Err(NUMBER.replace("{slide}", "4").replace("{count}", "3")));
        assert_eq!(checked("move", &owned(&["1", "3"]), 3), Ok(["move".into(), "0".into(), "2".into()]));
        assert_eq!(checked("move", &owned(&["1", "4"]), 3), Err(POSITION.replace("{position}", "4").replace("{last}", "3")));
    }
}
