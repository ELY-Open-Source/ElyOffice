//! Pictures dropped or pasted, copied beside the file (MARK-07).

use std::{
    io,
    path::{Path, PathBuf},
};

use ely_gpui_component::documents::Attachment;
use ely_ui::i18n::text;
use gpui::{App, Context, ImageFormat};

use crate::view::{Failed, MarkView, Mode};

const PICTURES: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"];

/// What goes into `assets/`: a file, or pasted bytes.
enum Source {
    File(PathBuf),
    Bytes(String, Vec<u8>),
}

fn said(key: &str, cx: &App) -> Failed {
    Failed(text(cx, key, &[]).to_string())
}

impl MarkView {
    /// Copies pictures off the window, then links them.
    pub(crate) fn attach(&mut self, attached: &Attachment, cx: &mut Context<Self>) {
        if self.mode == Mode::Preview {
            return cx.emit(said("mark.attach.preview", cx));
        }
        let Some(folder) = self.folder.clone() else {
            return cx.emit(said("mark.attach.untitled", cx));
        };
        let sources: Vec<Source> = match attached {
            Attachment::Files(paths) => {
                let (pictures, others): (Vec<&PathBuf>, Vec<&PathBuf>) =
                    paths.iter().partition(|path| picture(path));
                if !others.is_empty() {
                    cx.emit(said("mark.attach.images", cx));
                }
                // A link names the file; its name must be text.
                let (pictures, unnamed): (Vec<&PathBuf>, Vec<&PathBuf>) = pictures
                    .into_iter()
                    .partition(|path| path.file_name().is_some_and(|name| name.to_str().is_some()));
                if !unnamed.is_empty() {
                    cx.emit(said("mark.attach.name", cx));
                }
                pictures.into_iter().cloned().map(Source::File).collect()
            }
            Attachment::Picture(image) => {
                let millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the clock is past 1970")
                    .as_millis();
                let name = format!("pasted-{millis}.{}", extension(image.format()));
                vec![Source::Bytes(name, image.bytes().to_vec())]
            }
        };
        if sources.is_empty() {
            return;
        }
        log::info!("mark: {} pictures copy in", sources.len());
        let assets = folder.join("assets");
        cx.spawn(async move |view, cx| {
            let copied = cx
                .background_executor()
                .spawn(async move {
                    sources
                        .into_iter()
                        .map(|source| match source {
                            Source::File(path) => ely_files::import_attachment(&path, &assets),
                            Source::Bytes(name, bytes) => {
                                ely_files::write_attachment(&name, &bytes, &assets)
                            }
                        })
                        .collect::<io::Result<Vec<PathBuf>>>()
                })
                .await;
            view.update(cx, |view, cx| view.attached(folder, copied, cx))
                .ok();
        })
        .detach();
    }

    /// Links the copies, unless the file moved meanwhile.
    fn attached(
        &mut self,
        folder: PathBuf,
        copied: io::Result<Vec<PathBuf>>,
        cx: &mut Context<Self>,
    ) {
        let copied = match copied {
            Ok(copied) => copied,
            Err(error) => {
                log::error!("mark: a picture did not copy: {error}");
                let said = text(cx, "office.error.engine", &[("error", &error.to_string())]);
                return cx.emit(Failed(said.to_string()));
            }
        };
        if self.folder.as_ref() != Some(&folder) {
            log::info!("mark: the file moved while pictures copied");
            let assets = folder.join("assets").display().to_string();
            let said = text(cx, "mark.attach.moved", &[("folder", &assets)]);
            return cx.emit(Failed(said.to_string()));
        }
        let links: Vec<String> = copied.iter().map(|copy| link(&folder, copy)).collect();
        let written = self
            .document()
            .clone()
            .update(cx, |document, cx| document.insert(&links.join("\n"), cx));
        if !written {
            cx.emit(said("mark.attach.preview", cx));
        }
    }
}

fn picture(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| PICTURES.contains(&ext.to_string_lossy().to_ascii_lowercase().as_str()))
}

fn extension(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Gif => "gif",
        ImageFormat::Webp => "webp",
        ImageFormat::Svg => "svg",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Tiff => "tiff",
        ImageFormat::Ico => "ico",
        ImageFormat::Pnm => "pnm",
    }
}

/// The Markdown link to `copy`, relative to `folder`.
fn link(folder: &Path, copy: &Path) -> String {
    let relative = copy.strip_prefix(folder).expect("copied under the folder");
    let address: Vec<String> = relative
        .components()
        .map(|part| escaped(part.as_os_str().to_str().expect("named in text")))
        .collect();
    let stem = copy
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .expect("named in text");
    // A line break would end the image.
    let alt = stem
        .replace(|ch: char| ch.is_control(), " ")
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]");
    format!("![{alt}]({})", address.join("/"))
}

/// A path part, every byte but plain ones escaped.
fn escaped(part: &str) -> String {
    part.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, path::Path, rc::Rc};

    use ely_gpui_component::documents::{Attachment, markdown_tree, mdast::Node};
    use gpui::{Entity, Image, ImageFormat, TestAppContext, VisualTestContext};

    use super::link;
    use crate::desk::{
        Desk,
        tests::{open, start},
    };
    use crate::view::{Failed, MarkView};

    /// What the view says, in order.
    type Said = Rc<RefCell<Vec<String>>>;

    /// A saved file open in ElyMark, and what it says.
    fn saved(
        cx: &mut TestAppContext,
    ) -> (
        Entity<MarkView>,
        &'static mut VisualTestContext,
        [tempfile::TempDir; 2],
        Said,
    ) {
        let (data, files) = (
            tempfile::tempdir().expect("data"),
            tempfile::tempdir().expect("files"),
        );
        let path = files.path().join("a.md");
        std::fs::write(
            &path, "a
",
        )
        .expect("a file");
        let (desk, cx) = start(cx, data.path());
        open(&desk, &path, cx);
        let view = desk.read_with(cx, |desk: &Desk, _| desk.active_view().expect("a tab"));
        let said = Rc::new(RefCell::new(Vec::new()));
        let heard = said.clone();
        cx.update(|_, cx| {
            cx.subscribe(&view, move |_, failed: &Failed, _| {
                heard.borrow_mut().push(failed.0.clone())
            })
            .detach()
        });
        (view, cx, [data, files], said)
    }

    fn attach(view: &Entity<MarkView>, attached: Attachment, cx: &mut VisualTestContext) {
        let document = view.read_with(cx, |view, _| view.document().clone());
        document.update(cx, |_, cx| cx.emit(attached));
        cx.run_until_parked();
    }

    fn current(view: &Entity<MarkView>, cx: &mut VisualTestContext) -> String {
        view.read_with(cx, |view, cx| view.document().read(cx).text(cx))
    }

    #[gpui::test]
    fn a_dropped_picture_is_copied_beside_and_linked(cx: &mut TestAppContext) {
        let (view, cx, [_, files], said) = saved(cx);
        let shot = files.path().join("my shot.png");
        std::fs::write(&shot, "png").expect("a picture");
        attach(&view, Attachment::Files(vec![shot]), cx);
        assert!(files.path().join("assets/my shot.png").exists());
        assert!(
            current(&view, cx).contains("![my shot](assets/my%20shot.png)"),
            "{}",
            current(&view, cx)
        );
        assert!(said.borrow().is_empty());
    }

    #[gpui::test]
    fn a_pasted_picture_is_written_and_linked(cx: &mut TestAppContext) {
        let (view, cx, [_, files], _) = saved(cx);
        attach(
            &view,
            Attachment::Picture(Image::from_bytes(ImageFormat::Png, vec![1])),
            cx,
        );
        let assets: Vec<_> = std::fs::read_dir(files.path().join("assets"))
            .expect("assets")
            .collect();
        assert_eq!(assets.len(), 1);
        assert!(current(&view, cx).contains("](assets/pasted-"));
    }

    #[gpui::test]
    fn other_files_and_untitled_text_take_no_picture(cx: &mut TestAppContext) {
        let (view, cx, [_, files], said) = saved(cx);
        let note = files.path().join("note.txt");
        std::fs::write(&note, "text").expect("a file");
        attach(&view, Attachment::Files(vec![note]), cx);
        assert!(!files.path().join("assets").exists());
        assert_eq!(said.borrow().len(), 1, "other files are refused");
        view.update(cx, |view, _| view.folder = None);
        attach(
            &view,
            Attachment::Picture(Image::from_bytes(ImageFormat::Png, vec![1])),
            cx,
        );
        assert_eq!(said.borrow().len(), 2, "untitled text is refused");
        assert_eq!(
            current(&view, cx),
            "a
"
        );
    }

    #[gpui::test]
    fn a_file_moved_while_copying_gets_no_link(cx: &mut TestAppContext) {
        let (view, cx, [_, files], said) = saved(cx);
        let document = view.read_with(cx, |view, _| view.document().clone());
        document.update(cx, |_, cx| {
            cx.emit(Attachment::Picture(Image::from_bytes(
                ImageFormat::Png,
                vec![1],
            )))
        });
        let elsewhere = files.path().join("elsewhere");
        view.update(cx, |view, cx| view.moved(elsewhere, cx));
        cx.run_until_parked();
        assert_eq!(
            current(&view, cx),
            "a
"
        );
        assert_eq!(said.borrow().len(), 1, "the move is said");
    }

    #[test]
    fn links_escape_their_address_and_alt() {
        let folder = Path::new("/notes");
        let copy = folder.join("assets").join("a b[1].png");
        assert_eq!(
            link(folder, &copy),
            "![a b\\[1\\]](assets/a%20b%5B1%5D.png)"
        );
    }

    #[test]
    fn a_name_with_line_breaks_links_one_image() {
        let folder = Path::new("/notes");
        let written = link(
            folder,
            &folder.join("assets").join(
                "a

b.png",
            ),
        );
        let tree = markdown_tree(&written);
        let blocks = tree.children().expect("a root");
        let [Node::Paragraph(line)] = blocks.as_slice() else {
            panic!("one paragraph, not {blocks:?}")
        };
        assert!(
            matches!(line.children.as_slice(), [Node::Image(_)]),
            "{written}"
        );
    }

    /// A file name that is no text.
    #[cfg(all(unix, not(target_os = "macos")))]
    fn odd_name() -> std::ffi::OsString {
        use std::os::unix::ffi::OsStrExt as _;
        std::ffi::OsStr::from_bytes(b"pic-\xFF.png").to_owned()
    }

    #[cfg(windows)]
    fn odd_name() -> std::ffi::OsString {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[0x70, 0xD800, 0x2E, 0x70, 0x6E, 0x67])
    }

    // macOS takes only UTF-8 names: no such file.
    #[cfg(not(target_os = "macos"))]
    #[gpui::test]
    fn a_name_that_is_no_text_is_refused(cx: &mut TestAppContext) {
        let (view, cx, [_, files], said) = saved(cx);
        let odd = files.path().join(odd_name());
        std::fs::write(&odd, "png").expect("an oddly named picture");
        attach(&view, Attachment::Files(vec![odd]), cx);
        assert!(!files.path().join("assets").exists());
        assert_eq!(said.borrow().len(), 1);
        assert_eq!(
            current(&view, cx),
            "a
"
        );
    }
}
