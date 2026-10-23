//! Linux: the desktop portal's print dialog, over D-Bus.

use std::{
    collections::HashMap,
    fs::File,
    io::{Seek as _, Write as _},
    os::fd::{AsFd, FromRawFd as _},
    path::Path,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::{Context as _, anyhow, bail};
use futures::StreamExt as _;
use lopdf::Document;
use serde::Serialize;
use zbus::{
    Connection, Proxy,
    zvariant::{DynamicType, Fd, OwnedObjectPath, OwnedValue, Value},
};

const TITLE: &str = "Ely Office";
const DESKTOP: &str = "org.freedesktop.portal.Desktop";
const PORTAL: &str = "/org/freedesktop/portal/desktop";
/// Standard papers: name, label, width and height in millimetres.
const FORMS: [(&str, &str, f64, f64); 6] = [
    ("iso_a3", "A3", 297.0, 420.0),
    ("iso_a4", "A4", 210.0, 297.0),
    ("iso_a5", "A5", 148.0, 210.0),
    ("iso_b5", "B5", 176.0, 250.0),
    ("na_letter", "US Letter", 215.9, 279.4),
    ("na_legal", "US Legal", 215.9, 355.6),
];

type Options = HashMap<&'static str, Value<'static>>;

mod choices;
mod sheets;

use crate::settings::Sides;

/// Shows the portal's print dialog for `pdf`; false when cancelled.
pub async fn print_pdf(pdf: &Path, sides: Sides) -> anyhow::Result<bool> {
    let sizes = {
        let pdf = pdf.to_owned();
        super::off(move || sizes(&pdf)).await?
    };
    let paper = super::paper_for(&sizes).ok_or_else(|| anyhow!("{} has no pages", pdf.display()))?;
    log::info!("print: paper {}x{} points", paper.0, paper.1);
    let bus = Connection::session().await.context("no session bus for the print portal")?;
    let portal = Proxy::new(&bus, DESKTOP, PORTAL, "org.freedesktop.portal.Print").await?;
    let settings = match duplex(sides) {
        Some(duplex) => Options::from([("duplex", Value::from(duplex))]),
        None => Options::new(),
    };
    log::info!("print: sides {sides:?}");
    let prepared = request(&bus, &portal, "PreparePrint", |mut options| {
        // We hand over PDF; the dialog must not offer others.
        options.insert("supported_output_file_formats", Value::from(vec!["pdf"]));
        ("", TITLE, settings, page_setup(paper), options)
    })
    .await?;
    let Some(mut prepared) = prepared else {
        log::info!("print: the dialog was cancelled");
        return Ok(false);
    };
    let token = prepared.get("token").context("the portal gave no print token")?.downcast_ref::<u32>()?;
    let mut part = |key: &str| -> anyhow::Result<HashMap<String, OwnedValue>> {
        Ok(prepared.remove(key).with_context(|| format!("the portal gave no {key}"))?.try_into()?)
    };
    let choices = choices::chosen(&part("settings")?, &part("page-setup")?)?;
    log::info!("print: {} copies, {}%, {} up", choices.copies, choices.scale * 100.0, choices.up);
    // The portal prints our bytes as given: we lay out.
    let laid = {
        let pdf = pdf.to_owned();
        super::off(move || sheets::compose(&pdf, &choices)).await?
    };
    let file = in_memory(&laid)?;
    let fd = Fd::from(file.as_fd());
    let printed = request(&bus, &portal, "Print", |mut options| {
        options.insert("token", Value::from(token));
        ("", TITLE, fd, options)
    })
    .await?;
    log::info!("print: the portal took the job, printed {}", printed.is_some());
    Ok(printed.is_some())
}

/// GTK calls long-edge binding horizontal.
fn duplex(sides: Sides) -> Option<&'static str> {
    match sides {
        Sides::Printer => None,
        Sides::One => Some("simplex"),
        Sides::Long => Some("horizontal"),
        Sides::Short => Some("vertical"),
    }
}

/// Calls `method`, awaits its answer; None when cancelled.
async fn request<B: Serialize + DynamicType>(
    bus: &Connection,
    portal: &Proxy<'_>,
    method: &str,
    body: impl FnOnce(Options) -> B,
) -> anyhow::Result<Option<HashMap<String, OwnedValue>>> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let token = format!("ely{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
    let sender = bus.unique_name().context("the bus gave no name")?.trim_start_matches(':').replace('.', "_");
    let path = format!("{PORTAL}/request/{sender}/{token}");
    // Listen before asking: the answer may come first.
    let request = Proxy::new(bus, DESKTOP, path.as_str(), "org.freedesktop.portal.Request").await?;
    let mut answers = request.receive_signal("Response").await?;
    let handle: OwnedObjectPath = portal.call(method, &body(Options::from([("handle_token", Value::from(token.clone()))]))).await?;
    if handle.as_str() != path {
        bail!("the portal answers at {handle}, not {path}");
    }
    let answer = answers.next().await.context("the portal closed the request")?;
    let (code, results): (u32, HashMap<String, OwnedValue>) = answer.body().deserialize()?;
    match code {
        0 => Ok(Some(results)),
        1 => Ok(None),
        _ => bail!("the portal ended {method} ({code})"),
    }
}

/// The portal's page setup for a paper, margins zero.
fn page_setup((width, height): (f64, f64)) -> Options {
    let millimetres = |points: f64| points * 25.4 / 72.0;
    let (short, long) = (millimetres(width.min(height)), millimetres(width.max(height)));
    let points = |mm: f64| mm * 72.0 / 25.4;
    let page = (width.min(height), width.max(height));
    let (name, label) = match FORMS.iter().find(|(.., across, down)| super::holds((points(*across), points(*down)), page)) {
        Some((name, label, ..)) => (name.to_string(), label.to_string()),
        None => (format!("custom_{short:.0}x{long:.0}mm"), format!("{short:.0} × {long:.0} mm")),
    };
    let orientation = if width > height { "landscape" } else { "portrait" };
    Options::from([
        ("Name", Value::from(name)),
        ("DisplayName", Value::from(label)),
        ("Width", Value::from(short)),
        ("Height", Value::from(long)),
        ("MarginTop", Value::from(0.0)),
        ("MarginBottom", Value::from(0.0)),
        ("MarginLeft", Value::from(0.0)),
        ("MarginRight", Value::from(0.0)),
        ("Orientation", Value::from(orientation)),
    ])
}

/// Each page's size in points, as it shows.
fn sizes(pdf: &Path) -> anyhow::Result<Vec<(f64, f64)>> {
    let document = Document::load(pdf)?;
    document.get_pages().into_values().map(|page| Ok(sheets::Shown::of(&document, page)?.size())).collect()
}

/// `bytes` in an unnamed file, read from its start.
fn in_memory(bytes: &[u8]) -> anyhow::Result<File> {
    let fd = unsafe { libc::memfd_create(c"ely-print".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        bail!("no file for the print job: {}", std::io::Error::last_os_error());
    }
    let mut file = unsafe { File::from_raw_fd(fd) };
    file.write_all(bytes)?;
    file.rewind()?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::{page_setup, sizes};

    #[test]
    fn a_known_paper_goes_by_its_name() {
        let setup = page_setup((842.0, 595.0));
        let text = |key: &str| setup[key].downcast_ref::<&str>().expect("text");
        let number = |key: &str| setup[key].downcast_ref::<f64>().expect("a number").round();
        assert_eq!((text("Name"), text("Orientation")), ("iso_a4", "landscape"));
        assert_eq!((number("Width"), number("Height"), number("MarginLeft")), (210.0, 297.0, 0.0));
        let custom = page_setup((283.5, 425.2));
        assert_eq!(custom["Name"].downcast_ref::<&str>().expect("text"), "custom_100x150mm");
        assert_eq!(custom["Orientation"].downcast_ref::<&str>().expect("text"), "portrait");
    }

    #[test]
    fn page_sizes_inherit_turn_and_scale() {
        use lopdf::{Document, Object, dictionary};

        let mut document = Document::with_version("1.7");
        let tree = document.new_object_id();
        let media: Vec<Object> = vec![0.into(), 0.into(), 595.into(), 842.into()];
        let upright = document.add_object(dictionary! { "Type" => "Page", "Parent" => tree });
        let turned = document.add_object(dictionary! { "Type" => "Page", "Parent" => tree, "Rotate" => 90 });
        let cropped = document.add_object(dictionary! { "Type" => "Page", "Parent" => tree, "CropBox" => vec![10.into(), 10.into(), 110.into(), 60.into()] });
        let large = document.add_object(dictionary! { "Type" => "Page", "Parent" => tree, "UserUnit" => 2 });
        let kids: Vec<Object> = vec![upright.into(), turned.into(), cropped.into(), large.into()];
        document.objects.insert(tree, dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 4, "MediaBox" => media }.into());
        let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => tree });
        document.trailer.set("Root", catalog);
        let folder = tempfile::tempdir().expect("a folder");
        let pdf = folder.path().join("pages.pdf");
        document.save(&pdf).expect("saves");
        assert_eq!(sizes(&pdf).expect("reads"), [(595.0, 842.0), (842.0, 595.0), (100.0, 50.0), (1190.0, 1684.0)]);
    }
}
