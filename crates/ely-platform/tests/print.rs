//! Printing through a Windows file printer and fake Linux portal.

#[cfg(not(target_os = "macos"))]
use std::path::Path;

/// Pages of `sizes` points, each with an inch square.
#[cfg(not(target_os = "macos"))]
fn fixture(path: &Path, sizes: &[(f32, f32)]) {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.7");
    let tree = document.new_object_id();
    let kids: Vec<Object> = sizes
        .iter()
        .map(|&(width, height)| {
            let square = format!("0 g 72 {} 72 72 re f", height - 144.0);
            let content = document.add_object(Stream::new(dictionary! {}, square.into_bytes()));
            let media: Vec<Object> = vec![0.into(), 0.into(), width.into(), height.into()];
            document
                .add_object(dictionary! { "Type" => "Page", "Parent" => tree, "MediaBox" => media, "Contents" => content })
                .into()
        })
        .collect();
    let count = kids.len() as i64;
    document.objects.insert(tree, dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => count }.into());
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => tree });
    document.trailer.set("Root", catalog);
    document.save(path).expect("the fixture saves");
}

#[cfg(windows)]
fn main() {
    let folder = tempfile::tempdir().expect("a folder");
    unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) }.expect("the runtime");
    // A5, then wide A5: the paper stays upright.
    let mixed = printed(folder.path(), "mixed", &[(420.0, 595.0), (595.0, 420.0)]);
    assert_eq!(papers(&mixed), [(420, 595), (420, 595)]);
    let square = |page| windows_page::dark(&mixed, page, (108, 108));
    assert_eq!((square(0), square(1)), (true, false), "the upright page keeps its corner");
    // Turned left, its top left lands bottom left.
    assert!(windows_page::dark(&mixed, 1, (108, 487)), "the turned page's square");
    assert!(!windows_page::dark(&mixed, 0, (312, 108)), "nothing at the upright page's other corner");
    // A4 landscape alone turns the paper.
    let wide = printed(folder.path(), "wide", &[(842.0, 595.0)]);
    assert_eq!(papers(&wide), [(842, 595)]);
    assert!(windows_page::dark(&wide, 0, (108, 108)), "the wide page keeps its corner");
    // A paper the printer lacks: refused, nothing cut or written.
    let source = folder.path().join("custom.pdf");
    fixture(&source, &[(1000.0, 600.0)]);
    let job = folder.path().join("custom-job.pdf");
    unsafe { std::env::set_var("ELY_PRINT_TO", &job) };
    let error = futures::executor::block_on(ely_platform::print::print_pdf(&source, ely_platform::settings::Sides::Printer)).expect_err("no such paper");
    assert!(error.to_string().contains("page 1 is 353 × 212 mm; the printer's paper is"), "{error}");
    assert!(!job.exists(), "nothing printed");
    // 1.6 mm past A4: refused, not cut.
    fixture(&source, &[(600.0, 842.0)]);
    let error = futures::executor::block_on(ely_platform::print::print_pdf(&source, ely_platform::settings::Sides::Printer)).expect_err("past A4");
    assert!(error.to_string().contains("page 1 is 212 × 297 mm; the printer's paper is"), "{error}");
    assert!(!job.exists(), "nothing printed");
    println!("print: windows ok");
}

/// Prints `sizes` through the file printer; the job's PDF.
#[cfg(windows)]
fn printed(folder: &Path, name: &str, sizes: &[(f32, f32)]) -> std::path::PathBuf {
    let source = folder.join(format!("{name}.pdf"));
    fixture(&source, sizes);
    let job = folder.join(format!("{name}-job.pdf"));
    // Before any thread starts: the job goes to a file.
    unsafe { std::env::set_var("ELY_PRINT_TO", &job) };
    assert!(futures::executor::block_on(ely_platform::print::print_pdf(&source, ely_platform::settings::Sides::Printer)).expect("prints"), "printed");
    job
}

/// Each printed page's paper in whole points.
#[cfg(windows)]
fn papers(job: &Path) -> Vec<(i64, i64)> {
    let document = lopdf::Document::load(job).expect("the job is a PDF");
    document
        .get_pages()
        .into_values()
        .map(|page| {
            let media = document.get_dictionary(page).and_then(|page| page.get(b"MediaBox")).expect("a media box");
            let corners: Vec<f32> = media.as_array().expect("an array").iter().map(|n| n.as_float().expect("a number")).collect();
            ((corners[2] - corners[0]).round() as i64, (corners[3] - corners[1]).round() as i64)
        })
        .collect()
}

/// Reads printed pages back through Windows' own renderer.
#[cfg(windows)]
mod windows_page {
    use std::path::Path;

    use windows::{
        Data::Pdf::{PdfDocument, PdfPageRenderOptions},
        Storage::{
            StorageFile,
            Streams::{DataReader, InMemoryRandomAccessStream},
        },
        core::HSTRING,
    };

    /// Whether `(x, y)` points from the top left is dark.
    pub fn dark(job: &Path, page: u32, (x, y): (u32, u32)) -> bool {
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(job)).and_then(|file| file.join()).expect("the job opens");
        let document = PdfDocument::LoadFromFileAsync(&file).and_then(|document| document.join()).expect("the job reads");
        let page = document.GetPage(page).expect("the page");
        let size = page.Size().expect("its size");
        let options = PdfPageRenderOptions::new().expect("options");
        // Asked in DIPs; scaled back to one pixel per point.
        options.SetDestinationWidth((size.Width * 0.75).round() as u32).expect("a width");
        let stream = InMemoryRandomAccessStream::new().expect("a stream");
        page.RenderWithOptionsToStreamAsync(&stream, &options).and_then(|render| render.join()).expect("renders");
        let length = stream.Size().expect("a size") as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).expect("a start")).expect("a reader");
        reader.LoadAsync(length).and_then(|load| load.join()).expect("loads");
        let mut png = vec![0; length as usize];
        reader.ReadBytes(&mut png).expect("reads");
        let image = image::load_from_memory(&png).expect("a PNG").to_luma8();
        let scale = f64::from(image.width()) / f64::from(size.Width * 0.75);
        let at = |point: u32| (f64::from(point) * scale) as u32;
        image.get_pixel(at(x), at(y)).0[0] < 64
    }
}

#[cfg(target_os = "linux")]
fn main() {
    use std::io::BufRead as _;

    // A private session bus: the stand-in portal and nothing else.
    let bus = std::process::Command::new("dbus-daemon")
        .args(["--session", "--nofork", "--nopidfile", "--print-address=1"])
        .stdout(std::process::Stdio::piped())
        .spawn();
    let Ok(mut bus) = bus else {
        println!("SKIP: no dbus-daemon");
        return;
    };
    let mut address = String::new();
    std::io::BufReader::new(bus.stdout.take().expect("its address")).read_line(&mut address).expect("reads the address");
    // Before any thread starts: the portal client finds this bus.
    unsafe { std::env::set_var("DBUS_SESSION_BUS_ADDRESS", address.trim()) };
    let folder = tempfile::tempdir().expect("a folder");
    let wide = folder.path().join("wide.pdf");
    fixture(&wide, &[(842.0, 595.0)]);
    let three = folder.path().join("three.pdf");
    fixture(&three, &[(595.0, 842.0), (420.0, 595.0), (300.0, 300.0)]);
    futures::executor::block_on(portal::check(&wide, &three));
    bus.kill().expect("the bus stops");
    println!("print: linux ok");
}

/// A stand-in for org.freedesktop.portal.Print.
#[cfg(target_os = "linux")]
mod portal {
    use std::{
        collections::HashMap,
        io::Read as _,
        path::Path,
        sync::{Arc, Mutex},
    };

    use ely_platform::settings::Sides;
    use zbus::{
        Connection,
        message::Header,
        zvariant::{OwnedFd, OwnedObjectPath, OwnedValue, Value},
    };

    #[derive(Default)]
    struct Seen {
        cancel: bool,
        /// What the dialog answers the user chose.
        choices: Vec<(&'static str, &'static str)>,
        setup: HashMap<String, OwnedValue>,
        /// The settings the dialog was opened with.
        settings: HashMap<String, OwnedValue>,
        formats: Option<Vec<String>>,
        token: Option<u32>,
        bytes: Option<Vec<u8>>,
    }

    struct Portal(Arc<Mutex<Seen>>);

    /// The request object the caller listens on.
    fn handle(header: &Header<'_>, options: &HashMap<String, OwnedValue>) -> OwnedObjectPath {
        let sender = header.sender().expect("a sender").trim_start_matches(':').replace('.', "_");
        let token: &str = options.get("handle_token").expect("a handle token").downcast_ref().expect("a string");
        OwnedObjectPath::try_from(format!("/org/freedesktop/portal/desktop/request/{sender}/{token}")).expect("a path")
    }

    async fn answer(connection: &Connection, handle: &OwnedObjectPath, code: u32, results: HashMap<&str, Value<'_>>) {
        connection
            .emit_signal(None::<()>, handle, "org.freedesktop.portal.Request", "Response", &(code, results))
            .await
            .expect("the response goes out");
    }

    #[zbus::interface(name = "org.freedesktop.portal.Print")]
    impl Portal {
        // The portal's own signature.
        #[allow(clippy::too_many_arguments)]
        async fn prepare_print(
            &self,
            #[zbus(header)] header: Header<'_>,
            #[zbus(connection)] connection: &Connection,
            _parent: &str,
            _title: &str,
            settings: HashMap<String, OwnedValue>,
            page_setup: HashMap<String, OwnedValue>,
            options: HashMap<String, OwnedValue>,
        ) -> OwnedObjectPath {
            let handle = handle(&header, &options);
            let formats = options.get("supported_output_file_formats").map(|formats| formats.try_clone().expect("a copy").try_into().expect("strings"));
            let (cancel, choices) = {
                let mut seen = self.0.lock().expect("the record");
                (seen.setup, seen.settings, seen.formats) = (page_setup.clone(), settings, formats);
                (seen.cancel, seen.choices.clone())
            };
            // GTK hands every setting back as text.
            let settings: HashMap<String, Value> = choices.into_iter().map(|(key, value)| (key.to_string(), Value::from(value))).collect();
            let results = match cancel {
                true => HashMap::new(),
                false => HashMap::from([
                    ("settings", Value::from(settings)),
                    ("page-setup", Value::from(page_setup)),
                    ("token", Value::from(7u32)),
                ]),
            };
            answer(connection, &handle, u32::from(cancel), results).await;
            handle
        }

        async fn print(
            &self,
            #[zbus(header)] header: Header<'_>,
            #[zbus(connection)] connection: &Connection,
            _parent: &str,
            _title: &str,
            fd: OwnedFd,
            options: HashMap<String, OwnedValue>,
        ) -> OwnedObjectPath {
            let handle = handle(&header, &options);
            let mut bytes = Vec::new();
            std::fs::File::from(std::os::fd::OwnedFd::from(fd)).read_to_end(&mut bytes).expect("the job reads");
            let token = options.get("token").map(|token| token.downcast_ref::<u32>().expect("a number"));
            {
                let mut seen = self.0.lock().expect("the record");
                (seen.token, seen.bytes) = (token, Some(bytes));
            }
            answer(connection, &handle, 0, HashMap::new()).await;
            handle
        }

        #[zbus(property, name = "version")]
        fn version(&self) -> u32 {
            3
        }
    }

    /// A sheet's size, then each form's width and placed corners.
    type Sheet = ((i64, i64), Vec<(i64, [i64; 4])>);

    fn sheets(bytes: &[u8]) -> Vec<Sheet> {
        use lopdf::{Document, Object, content::Content};

        let document = Document::load_mem(bytes).expect("a PDF");
        let numbers = |object: &Object| -> Vec<f64> { object.as_array().expect("numbers").iter().map(|n| f64::from(n.as_float().expect("a number"))).collect() };
        let round = |n: f64| n.round() as i64;
        let pages = document.get_pages().into_values().map(|page| {
            let dict = document.get_dictionary(page).expect("a page");
            let media = numbers(dict.get(b"MediaBox").expect("a box"));
            let resources = dict.get(b"Resources").and_then(Object::as_dict).expect("resources");
            let forms = resources.get(b"XObject").and_then(Object::as_dict).expect("forms");
            let content = Content::decode(&document.get_page_content(page)).expect("content");
            let mut m = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            let mut placed = Vec::new();
            for op in content.operations {
                match op.operator.as_str() {
                    "cm" => m = op.operands.iter().map(|n| f64::from(n.as_float().expect("a number"))).collect(),
                    "Do" => {
                        let form = forms.get(op.operands[0].as_name().expect("a name")).and_then(Object::as_reference).expect("a form");
                        let stream = document.get_object(form).and_then(Object::as_stream).expect("a stream");
                        let b = numbers(stream.dict.get(b"BBox").expect("a box"));
                        let at = |x: f64, y: f64| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
                        let ((x0, y0), (x1, y1)) = (at(b[0], b[1]), at(b[2], b[3]));
                        placed.push((round(b[2] - b[0]), [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)].map(round)));
                    }
                    _ => {}
                }
            }
            ((round(media[2]), round(media[3])), placed)
        });
        pages.collect()
    }

    pub async fn check(source: &Path, three: &Path) {
        let seen = Arc::new(Mutex::new(Seen::default()));
        let _portal = zbus::connection::Builder::session()
            .and_then(|builder| builder.name("org.freedesktop.portal.Desktop"))
            .and_then(|builder| builder.serve_at("/org/freedesktop/portal/desktop", Portal(seen.clone())))
            .expect("a portal")
            .build()
            .await
            .expect("the portal serves");
        assert!(ely_platform::print::print_pdf(source, Sides::Printer).await.expect("prints"), "printed");
        {
            let seen = seen.lock().expect("the record");
            let text = |key: &str| seen.setup.get(key).map(|value| value.downcast_ref::<String>().expect("text"));
            let number = |key: &str| seen.setup.get(key).map(|value| value.downcast_ref::<f64>().expect("a number").round());
            assert_eq!(text("Name").as_deref(), Some("iso_a4"));
            assert_eq!(text("Orientation").as_deref(), Some("landscape"));
            assert_eq!((number("Width"), number("Height"), number("MarginTop")), (Some(210.0), Some(297.0), Some(0.0)));
            assert_eq!(seen.token, Some(7), "the print carries the dialog's token");
            assert!(seen.settings.is_empty(), "the printer's own sides stay");
            assert_eq!(seen.formats.as_deref(), Some(["pdf".to_string()].as_slice()), "PDF is all we hand over");
            let printed = sheets(seen.bytes.as_deref().expect("a job"));
            assert_eq!(printed, [((842, 595), vec![(842, [0, 0, 842, 595])])], "the page fills its sheet");
        }
        // GTK's words: horizontal is the long edge.
        for (sides, duplex) in [(Sides::Long, "horizontal"), (Sides::Short, "vertical"), (Sides::One, "simplex")] {
            *seen.lock().expect("the record") = Seen::default();
            assert!(ely_platform::print::print_pdf(source, sides).await.expect("prints"), "printed");
            let seen = seen.lock().expect("the record");
            let asked = seen.settings.get("duplex").map(|value| value.downcast_ref::<String>().expect("text"));
            assert_eq!(asked.as_deref(), Some(duplex), "{sides:?}");
        }
        // Page two at half size, from the sheet's top left.
        let choices = vec![("print-pages", "ranges"), ("page-ranges", "1"), ("scale", "50")];
        *seen.lock().expect("the record") = Seen { choices, ..Seen::default() };
        assert!(ely_platform::print::print_pdf(three, Sides::Printer).await.expect("prints"), "printed");
        let printed = sheets(seen.lock().expect("the record").bytes.as_deref().expect("a job"));
        assert_eq!(printed, [((595, 842), vec![(420, [0, 545, 210, 842])])]);
        // Two copies, uncollated, of the last two pages, in reverse.
        let choices = vec![("print-pages", "ranges"), ("page-ranges", "1--1"), ("reverse", "true"), ("n-copies", "2"), ("collate", "false")];
        *seen.lock().expect("the record") = Seen { choices, ..Seen::default() };
        assert!(ely_platform::print::print_pdf(three, Sides::Printer).await.expect("prints"), "printed");
        let printed = sheets(seen.lock().expect("the record").bytes.as_deref().expect("a job"));
        let widths = |printed: &[Sheet]| -> Vec<i64> { printed.iter().map(|(_, placed)| placed.first().map_or(0, |form| form.0)).collect() };
        assert_eq!(widths(&printed), [300, 300, 420, 420]);
        // Two-sided copies each start a sheet; 0 is blank.
        for (collate, sent) in [("true", [595, 420, 300, 0, 595, 420, 300, 0]), ("false", [595, 420, 595, 420, 300, 0, 300, 0])] {
            let choices = vec![("duplex", "horizontal"), ("n-copies", "2"), ("collate", collate)];
            *seen.lock().expect("the record") = Seen { choices, ..Seen::default() };
            assert!(ely_platform::print::print_pdf(three, Sides::Long).await.expect("prints"), "printed");
            let printed = sheets(seen.lock().expect("the record").bytes.as_deref().expect("a job"));
            assert_eq!(widths(&printed), sent, "collate {collate}");
        }
        // Cancelled, nothing prints.
        *seen.lock().expect("the record") = Seen { cancel: true, ..Seen::default() };
        assert!(!ely_platform::print::print_pdf(source, Sides::Printer).await.expect("cancels"), "not printed");
        assert_eq!(seen.lock().expect("the record").bytes, None, "no job went out");
    }
}

/// macOS prints through tests/scripts/print.sh, in a real window.
#[cfg(target_os = "macos")]
fn main() {}
