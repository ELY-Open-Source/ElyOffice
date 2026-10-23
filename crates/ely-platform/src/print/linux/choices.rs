//! What the portal's dialog chose: its settings and page setup.

use std::collections::HashMap;

use anyhow::{Context as _, bail};
use zbus::zvariant::OwnedValue;

type Map = HashMap<String, OwnedValue>;

/// Which sheets print, counting from one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Set {
    All,
    Even,
    Odd,
}

/// How several pages fill one sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Order {
    pub across_first: bool,
    pub leftward: bool,
    pub upward: bool,
}

/// The dialog's choices; the app applies every one.
#[derive(Debug, PartialEq)]
pub(super) struct Choices {
    /// Pages from zero, both ends kept; None is every page.
    pub ranges: Option<Vec<(u32, u32)>>,
    pub set: Set,
    pub reverse: bool,
    pub copies: u32,
    pub collate: bool,
    /// Sheets print front and back.
    pub duplex: bool,
    pub scale: f64,
    pub up: u32,
    pub order: Order,
    /// The sheet as it lies, in points.
    pub sheet: (f64, f64),
    pub upside_down: bool,
}

/// GTK's settings and page setup, as the portal passes them.
pub(super) fn chosen(settings: &Map, setup: &Map) -> anyhow::Result<Choices> {
    let ranges = match text(settings, "print-pages")?.unwrap_or("all") {
        "all" => None,
        "ranges" => Some(ranges(text(settings, "page-ranges")?.context("the print dialog gave no page ranges")?)?),
        other => bail!("the print dialog chose the {other} pages; Ely offers none"),
    };
    let set = match text(settings, "page-set")?.unwrap_or("all") {
        "all" => Set::All,
        "even" => Set::Even,
        "odd" => Set::Odd,
        other => bail!("the print dialog chose the page set {other}"),
    };
    // GTK's CUPS backend sends even or odd to CUPS.
    let set = match settings.contains_key("output-uri") || settings.contains_key("lpr-commandline") {
        true => set,
        false => Set::All,
    };
    let scale = number(settings, "scale", 100.0)? / 100.0;
    if !(scale.is_finite() && scale > 0.0) {
        bail!("the print dialog chose a scale of {}%", scale * 100.0);
    }
    let order = order(text(settings, "number-up-layout")?.unwrap_or("lrtb"))?;
    let millimetres = |key: &str| -> anyhow::Result<f64> {
        let value = setup.get(key).with_context(|| format!("the page setup gives no {key}"))?;
        Ok(value.downcast_ref::<f64>().with_context(|| format!("the page setup's {key} is no number"))? * 72.0 / 25.4)
    };
    let (width, height) = (millimetres("Width")?, millimetres("Height")?);
    let way = setup.get("Orientation").context("the page setup gives no orientation")?;
    let (sheet, upside_down) = match way.downcast_ref::<&str>().context("the page setup's orientation is no text")? {
        "portrait" => ((width, height), false),
        "landscape" => ((height, width), false),
        "reverse-portrait" => ((width, height), true),
        "reverse-landscape" => ((height, width), true),
        other => bail!("the page setup's orientation is {other}"),
    };
    Ok(Choices {
        ranges,
        set,
        reverse: flag(settings, "reverse", false)?,
        copies: count(settings, "n-copies")?,
        collate: flag(settings, "collate", true)?,
        duplex: match text(settings, "duplex")?.unwrap_or("simplex") {
            "simplex" => false,
            "horizontal" | "vertical" => true,
            other => bail!("the print dialog chose the sides {other}"),
        },
        scale,
        up: count(settings, "number-up")?,
        order,
        sheet,
        upside_down,
    })
}

/// GTK's ranges, "0-2,4" from zero; end -1 runs on.
fn ranges(text: &str) -> anyhow::Result<Vec<(u32, u32)>> {
    let page = |part: &str| part.trim().parse::<u32>().with_context(|| format!("the page range {text} is unreadable"));
    let ranges = text
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (from, to) = match part.split_once('-') {
                Some((from, to)) if to.trim() == "-1" => (page(from)?, u32::MAX),
                Some((from, to)) => (page(from)?, page(to)?),
                None => (page(part)?, page(part)?),
            };
            if from > to {
                bail!("the page range {text} runs backwards");
            }
            Ok((from, to))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    if ranges.is_empty() {
        bail!("the page ranges are empty");
    }
    Ok(ranges)
}

/// GTK's n-up nick, such as "lrtb": first way, then next.
fn order(nick: &str) -> anyhow::Result<Order> {
    let (first, then) = nick.split_at_checked(2).with_context(|| format!("the n-up layout {nick} is unknown"))?;
    let across = |way: &str| matches!(way, "lr" | "rl");
    let down = |way: &str| matches!(way, "tb" | "bt");
    if !(across(first) && down(then) || down(first) && across(then)) {
        bail!("the n-up layout {nick} is unknown");
    }
    Ok(Order { across_first: across(first), leftward: nick.contains("rl"), upward: nick.contains("bt") })
}

fn text<'a>(settings: &'a Map, key: &str) -> anyhow::Result<Option<&'a str>> {
    let value = settings.get(key).map(|value| value.downcast_ref::<&str>());
    value.transpose().with_context(|| format!("the print setting {key} is no text"))
}

fn number(settings: &Map, key: &str, unset: f64) -> anyhow::Result<f64> {
    match text(settings, key)? {
        Some(text) => text.trim().parse().with_context(|| format!("the print setting {key} is {text}")),
        None => Ok(unset),
    }
}

/// A whole number, at least one; one when unset.
fn count(settings: &Map, key: &str) -> anyhow::Result<u32> {
    let count = match text(settings, key)? {
        Some(text) => text.trim().parse().with_context(|| format!("the print setting {key} is {text}"))?,
        None => 1,
    };
    if count == 0 {
        bail!("the print setting {key} is zero");
    }
    Ok(count)
}

fn flag(settings: &Map, key: &str, unset: bool) -> anyhow::Result<bool> {
    match text(settings, key)? {
        None => Ok(unset),
        Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(other) => bail!("the print setting {key} is {other}"),
    }
}

#[cfg(test)]
mod tests {
    use zbus::zvariant::{OwnedValue, Value};

    use super::{Choices, Map, Order, Set, chosen};

    fn map(pairs: &[(&str, Value<'static>)]) -> Map {
        pairs.iter().map(|(key, value)| (key.to_string(), OwnedValue::try_from(value.clone()).expect("owned"))).collect()
    }

    fn a4(orientation: &'static str) -> Map {
        map(&[("Width", Value::from(210.0)), ("Height", Value::from(297.0)), ("Orientation", Value::from(orientation))])
    }

    #[test]
    fn unset_settings_print_everything_once() {
        let choices = chosen(&Map::new(), &a4("portrait")).expect("reads");
        let order = Order { across_first: true, leftward: false, upward: false };
        let (width, height) = (210.0 * 72.0 / 25.4, 297.0 * 72.0 / 25.4);
        assert_eq!(
            choices,
            Choices {
                ranges: None,
                set: Set::All,
                reverse: false,
                copies: 1,
                collate: true,
                duplex: false,
                scale: 1.0,
                up: 1,
                order,
                sheet: (width, height),
                upside_down: false
            }
        );
    }

    #[test]
    fn gtk_settings_read_as_given() {
        let settings = map(&[
            ("print-pages", Value::from("ranges")),
            ("page-ranges", Value::from("0-1, 4,7--1")),
            ("page-set", Value::from("even")),
            ("reverse", Value::from("true")),
            ("n-copies", Value::from("3")),
            ("collate", Value::from("false")),
            ("duplex", Value::from("vertical")),
            ("scale", Value::from("50")),
            ("number-up", Value::from("4")),
            ("number-up-layout", Value::from("btrl")),
            ("output-uri", Value::from("file:///tmp/out.pdf")),
        ]);
        let choices = chosen(&settings, &a4("reverse-landscape")).expect("reads");
        assert_eq!(choices.ranges, Some(vec![(0, 1), (4, 4), (7, u32::MAX)]));
        assert_eq!((choices.set, choices.reverse, choices.copies, choices.collate, choices.duplex), (Set::Even, true, 3, false, true));
        assert_eq!((choices.scale, choices.up), (0.5, 4));
        assert_eq!(choices.order, Order { across_first: false, leftward: true, upward: true });
        assert_eq!((choices.sheet.0.round(), choices.sheet.1.round(), choices.upside_down), (842.0, 595.0, true));
    }

    #[test]
    fn even_pages_are_ours_only_where_cups_leaves_them() {
        let set = |backend: &[(&str, Value<'static>)]| {
            let mut pairs = vec![("page-set", Value::from("even"))];
            pairs.extend_from_slice(backend);
            chosen(&map(&pairs), &a4("portrait")).expect("reads").set
        };
        // GTK's CUPS backend sends cups-page-set itself.
        assert_eq!(set(&[("printer", Value::from("Office"))]), Set::All);
        assert_eq!(set(&[("output-uri", Value::from("file:///tmp/out.pdf"))]), Set::Even);
        assert_eq!(set(&[("lpr-commandline", Value::from("lpr"))]), Set::Even);
    }

    #[test]
    fn what_cannot_be_honoured_fails() {
        let bad = |pairs: &[(&str, Value<'static>)]| chosen(&map(pairs), &a4("portrait")).is_err();
        assert!(bad(&[("print-pages", Value::from("selection"))]));
        assert!(bad(&[("print-pages", Value::from("ranges")), ("page-ranges", Value::from("two"))]));
        assert!(bad(&[("print-pages", Value::from("ranges")), ("page-ranges", Value::from("3-1"))]));
        assert!(chosen(&Map::new(), &a4("reverse_portrait")).is_err(), "GTK writes reverse-portrait");
        assert!(bad(&[("scale", Value::from("0"))]));
        assert!(bad(&[("n-copies", Value::from("0"))]));
        assert!(bad(&[("number-up-layout", Value::from("lrrl"))]));
        assert!(bad(&[("reverse", Value::from(1u32))]));
        assert!(bad(&[("duplex", Value::from("both"))]));
        assert!(chosen(&Map::new(), &map(&[("Width", Value::from(210.0))])).is_err(), "no height");
    }
}
