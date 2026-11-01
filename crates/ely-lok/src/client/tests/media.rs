use super::analysis::steps;
use super::show::open_deck;
use super::*;
use crate::show;

fn clip() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/media/clip.mp4")
}

fn media(worker: &mut Worker, doc: &Doc) -> Vec<show::Media> {
    show::media(&worker.values(doc, show::MEDIA).expect("reads")).expect("parses")
}

fn insert(worker: &mut Worker, doc: &Doc, path: &std::path::Path, link: bool) -> io::Result<()> {
    let url = crate::file_url(path);
    worker.run(
        doc,
        "media.insert",
        &[&url, &link.to_string(), "8000", "4500"],
    )
}

#[test]
fn embedded_media_goes_in_mid_slide_and_survives_a_save() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let before = steps(&mut worker, &doc);
    insert(&mut worker, &doc, &clip(), false).expect("inserts");
    assert_eq!(steps(&mut worker, &doc), before + 1);
    let list = media(&mut worker, &doc);
    assert_eq!(list.len(), 1);
    assert!(!list[0].link && !list[0].missing, "{:?}", list[0]);
    assert!(
        list[0].url.starts_with("vnd.sun.star.Package:"),
        "{}",
        list[0].url
    );
    let objects =
        show::objects(&worker.values(&doc, show::OBJECTS).expect("reads")).expect("parses");
    let object = &objects.objects[list[0].index];
    assert_eq!(object.kind, "media");
    let out = dir.path().join("media.odp");
    doc.save_as(&mut worker, &crate::file_url(&out), "odp")
        .expect("saves");
    let again = worker.load(&crate::file_url(&out)).expect("loads");
    let list = media(&mut worker, &again);
    assert_eq!(list.len(), 1);
    assert!(
        !list[0].link && !list[0].missing,
        "the copy plays after a reload: {:?}",
        list[0]
    );
}

#[test]
fn a_linked_file_that_goes_away_reads_as_missing() {
    if install().is_none() {
        eprintln!("SKIP: no pinned LOK build");
        return;
    }
    let (dir, mut worker, doc) = open_deck();
    let copy = dir.path().join("linked.mp4");
    std::fs::copy(clip(), &copy).expect("copies");
    insert(&mut worker, &doc, &copy, true).expect("inserts");
    let list = media(&mut worker, &doc);
    assert!(list[0].link && !list[0].missing, "{:?}", list[0]);
    std::fs::remove_file(&copy).expect("removes");
    assert!(media(&mut worker, &doc)[0].missing);
    let before = steps(&mut worker, &doc);
    let gone = dir.path().join("never.mp4");
    assert!(
        insert(&mut worker, &doc, &gone, false).is_err(),
        "nothing to embed"
    );
    assert_eq!(steps(&mut worker, &doc), before);
}

/// Class names of the windows process `pid` owns.
#[cfg(windows)]
fn windows_of(pid: u32) -> Vec<String> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowExW, GetClassNameW, GetWindowThreadProcessId, HWND_MESSAGE,
    };
    unsafe extern "system" fn each(w: HWND, all: isize) -> i32 {
        unsafe { (*(all as *mut Vec<HWND>)).push(w) };
        1
    }
    let mut all: Vec<HWND> = Vec::new();
    unsafe { EnumWindows(Some(each), &mut all as *mut Vec<HWND> as isize) };
    let mut w: HWND = std::ptr::null_mut();
    loop {
        w = unsafe { FindWindowExW(HWND_MESSAGE, w, std::ptr::null(), std::ptr::null()) };
        if w.is_null() {
            break;
        }
        all.push(w);
    }
    all.into_iter()
        .filter(|w| {
            let mut owner = 0;
            unsafe { GetWindowThreadProcessId(*w, &mut owner) };
            owner == pid
        })
        .map(|w| {
            let mut name = [0u16; 256];
            let n = unsafe { GetClassNameW(w, name.as_mut_ptr(), 256) } as usize;
            String::from_utf16_lossy(&name[..n])
        })
        .collect()
}

/// A queued MFPlay callback outlived its player (R15.13a).
#[cfg(windows)]
#[test]
fn painting_a_video_slide_starts_no_player() {
    if install().is_none() {
        return eprintln!("SKIP: no pinned LOK build");
    }
    let (_dir, mut worker, doc) = open_deck();
    insert(&mut worker, &doc, &clip(), false).expect("inserts");
    doc.paint_part(&mut worker, (0, false), (320, 180)).expect("the thumbnail paints");
    doc.paint_tile(&mut worker, (0, 0, 15360, 15360), (512, 512)).expect("the slide paints");
    let players: Vec<String> = windows_of(worker.child.id())
        .into_iter()
        .filter(|class| class.starts_with("MFPlay."))
        .collect();
    assert!(players.is_empty(), "the kernel made a player: {players:?}");
}
