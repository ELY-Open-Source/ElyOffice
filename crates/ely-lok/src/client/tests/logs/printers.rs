//! SF.18: a document's own printer survives a save.

use super::*;

/// Lists printers; None when the tool is absent.
fn listed(program: &str, args: &[&str]) -> Option<std::process::Output> {
    match std::process::Command::new(program).args(args).output() {
        Ok(output) => Some(output),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("{program} runs: {error}"),
    }
}

/// The first printer this machine has, or None.
fn installed() -> Option<String> {
    if cfg!(windows) {
        let names = listed("powershell", &["-NoProfile", "-Command", "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-Printer | ForEach-Object Name"])?;
        assert!(names.status.success(), "Get-Printer: {}", String::from_utf8_lossy(&names.stderr));
        return String::from_utf8(names.stdout).expect("UTF-8 names").lines().map(str::trim).find(|name| !name.is_empty()).map(str::to_owned);
    }
    // lpstat speaks the system's language; read names and exits only.
    let names = listed("lpstat", &["-e"])?;
    assert!(names.status.success(), "lpstat -e: {}", String::from_utf8_lossy(&names.stderr));
    let names = String::from_utf8(names.stdout).expect("UTF-8 names");
    // Queues only: the kernel skips discovered printers.
    let queue = |name: &&str| listed("lpstat", &["-p", name]).is_some_and(|queue| queue.status.success());
    names.lines().map(str::trim).filter(|name| !name.is_empty()).find(queue).map(str::to_owned)
}

/// The value of the settings item `name`; empty when absent.
fn item(path: &Path, name: &str) -> String {
    let bytes = std::fs::read(path).expect("reads");
    let entries = super::super::zip::entries(&bytes);
    let (_, settings) = entries.iter().find(|(entry, _)| entry == "settings.xml").expect("settings.xml");
    let settings = String::from_utf8(settings.clone()).expect("UTF-8 settings");
    let Some(at) = settings.find(&format!(r#"config:name="{name}""#)) else {
        return String::new();
    };
    let rest = &settings[at..];
    let open = rest.find('>').expect("an open tag");
    if rest[..open].ends_with('/') {
        return String::new();
    }
    let text = &rest[open + 1..rest.find('<').expect("a close tag")];
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

/// `text` as XML character data.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The printer item Ely saves: no name.
pub(super) const UNNAMED: &str = r#"<config:config-item config:name="PrinterName" config:type="string"/>"#;

/// Loads `path` and saves it as `format` into `target`.
fn resaved(worker: &mut Worker, path: &Path, target: &Path, format: &str) {
    let doc = worker.load(&crate::file_url(path)).expect("loads");
    doc.save_as(worker, &crate::file_url(target), format).expect("saves");
    worker.close_doc(doc).expect("closes");
}

#[test]
fn a_named_printer_and_its_setup_survive_a_save() {
    let Some(install) = install() else {
        return eprintln!("SKIP: no pinned LOK build");
    };
    let Some(printer) = installed() else {
        return eprintln!("SKIP: no installed printer");
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("saver")).expect("the worker");
    let (text, deck) = (dir.path().join("text.fodt"), dir.path().join("deck.fodp"));
    std::fs::write(&text, FODT).expect("writes");
    std::fs::write(&deck, DECK).expect("writes");
    let (odt, odp) = (dir.path().join("plain.odt"), dir.path().join("plain.odp"));
    resaved(&mut worker, &text, &odt, "odt");
    resaved(&mut worker, &deck, &odp, "odp");
    worker.shutdown().expect("the worker ends");
    let named = |plain: &Path, file: &str| {
        let path = dir.path().join(file);
        let name = format!(r#"<config:config-item config:name="PrinterName" config:type="string">{}</config:config-item>"#, escaped(&printer));
        edited(plain, &path, &[("settings.xml", UNNAMED, &name)], None);
        path
    };
    // Decks name the printer before the layout.
    let metrics = dir.path().join("metrics-plain.odp");
    let low = r#"<config:config-item config:name="PrinterIndependentLayout" config:type="string">low-resolution"#;
    edited(&odp, &metrics, &[("settings.xml", low, &low.replace("low-resolution", "disabled"))], None);
    for (path, plain, format) in [(named(&odt, "named.odt"), &odt, "odt"), (named(&odp, "named.odp"), &odp, "odp"), (named(&metrics, "metrics.odp"), &odp, "odp")] {
        let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
        let (once, twice) = (path.with_extension(format!("1.{format}")), path.with_extension(format!("2.{format}")));
        resaved(&mut worker, &path, &once, format);
        resaved(&mut worker, &once, &twice, format);
        worker.shutdown().expect("the worker ends");
        for saved in [&once, &twice] {
            assert_eq!(item(saved, "PrinterName"), printer, "{} keeps its printer", saved.display());
        }
        let setup = item(&once, "PrinterSetup");
        assert!(!setup.is_empty() && setup != item(plain, "PrinterSetup"), "{} has its printer's setup", once.display());
        assert_eq!(item(&twice, "PrinterSetup"), setup, "{} keeps the same setup", twice.display());
    }
}
