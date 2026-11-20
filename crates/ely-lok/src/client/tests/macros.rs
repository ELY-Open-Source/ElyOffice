//! No macro runs, on load or by URL (LOK-11).

use super::*;

/// A document whose load and button run the marker writer.
fn macro_fodt(marker: &std::path::Path) -> String {
    let marker = marker.display().to_string().replace('\\', "/");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 xmlns:ooo="http://openoffice.org/2004/office"
 xmlns:form="urn:oasis:names:tc:opendocument:xmlns:form:1.0"
 xmlns:script="urn:oasis:names:tc:opendocument:xmlns:script:1.0"
 xmlns:xlink="http://www.w3.org/1999/xlink" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:scripts>
  <office:script script:language="ooo:Basic">
   <ooo:libraries>
    <ooo:library-embedded ooo:name="Standard">
     <ooo:module ooo:name="Module1">
      <ooo:source-code>Sub Sneaky&#x0A;  Open "{marker}" For Output As #1&#x0A;  Write #1, "ran"&#x0A;  Close #1&#x0A;End Sub</ooo:source-code>
     </ooo:module>
    </ooo:library-embedded>
   </ooo:libraries>
  </office:script>
  <office:event-listeners>
   <script:event-listener script:language="ooo:script" script:event-name="dom:load" xlink:href="vnd.sun.star.script:Standard.Module1.Sneaky?language=Basic&amp;location=document" xlink:type="simple"/>
  </office:event-listeners>
 </office:scripts>
 <office:body>
  <office:text>
   <office:forms form:automatic-focus="false" form:apply-design-mode="false">
    <form:form>
     <form:button form:name="btn" form:control-implementation="ooo:com.sun.star.form.component.CommandButton" xml:id="btn1" form:id="btn1" form:label="Run">
      <office:event-listeners>
       <script:event-listener script:language="ooo:script" script:event-name="form:performaction" xlink:href="vnd.sun.star.script:Standard.Module1.Sneaky?language=Basic&amp;location=document" xlink:type="simple"/>
      </office:event-listeners>
     </form:button>
    </form:form>
   </office:forms>
   <text:p>Macro doc</text:p>
  </office:text>
 </office:body>
</office:document>"#
    )
}

/// Initializes the profile fully, the CLI way (registers Basic).
pub(super) fn seed_basic(install: &Path, profile: &Path) {
    #[cfg(target_os = "macos")]
    let soffice = install.parent().expect("the app dir").join("MacOS/soffice");
    #[cfg(target_os = "linux")]
    let soffice = install.join("soffice.bin");
    #[cfg(windows)]
    let soffice = install.join("soffice.exe");
    let url = ipc::file_url_of(profile);
    let status = std::process::Command::new(&soffice)
        .args(["--headless", "--norestore", "--terminate_after_init"])
        .arg(format!("-env:UserInstallation={url}"))
        .status()
        .expect("soffice runs");
    assert!(status.success(), "the profile initializes");
}

/// The macro module: writes its marker through the UCB.
pub(super) fn module(marker: &Path) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE script:module PUBLIC \"-//OpenOffice.org//DTD OfficeDocument 1.0//EN\" \"module.dtd\">\n<script:module xmlns:script=\"http://openoffice.org/2000/script\" script:name=\"Module1\" script:language=\"StarBasic\">Sub Sneaky\n  oSFA = createUnoService(\"com.sun.star.ucb.SimpleFileAccess\")\n  oOut = oSFA.openFileWrite(\"{}\")\n  oOut.writeString(\"ran\")\n  oOut.closeOutput()\nEnd Sub\n</script:module>\n",
        marker.display().to_string().replace('\\', "/")
    )
}

pub(super) const MACRO_URL: &str = "macro:///Standard.Module1.Sneaky";

/// The document's own macro, by URL.
const DOCUMENT_MACRO_URL: &str = "macro://./Standard.Module1.Sneaky";

/// Loaded with macros enabled, the same call writes the marker.
#[test]
fn a_loaded_document_s_macro_never_runs() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let profile = dir.path().join("profile");
    let marker = dir.path().join("macro-ran.txt");
    seed_basic(&install, &profile);
    std::fs::write(profile.join("user/basic/Standard/Module1.xba"), module(&marker)).expect("the module writes");
    let mut worker = Worker::spawn_dev(&install, &profile).expect("the worker");
    let source = dir.path().join("macro.fodt");
    std::fs::write(&source, macro_fodt(&marker)).expect("the macro fodt writes");
    let doc = worker.load(&crate::file_url(&source)).expect("the macro document loads");
    // An OnLoad macro would write while events flow.
    poll_until(&mut worker, &doc, &Events::default(), std::time::Duration::from_secs(4), |_| marker.exists());
    assert!(!marker.try_exists().expect("the marker check"), "LOK-11: loading ran no macro");
    let ran = worker.run_macro(DOCUMENT_MACRO_URL).expect("the macro channel");
    assert_eq!(ran, 0, "LOK-11: the document's macros stay disabled");
    assert!(!marker.try_exists().expect("the marker check"), "LOK-11: the document's macro wrote nothing");
    let ran = worker.run_macro(MACRO_URL).expect("the macro channel");
    assert!(ran != 0, "the build runs the library macro when asked");
    assert!(marker.try_exists().expect("the marker check"), "the sample is real: it writes when run");
    worker.close_doc(doc).expect("the document closes");
}

#[test]
fn a_production_worker_refuses_run_macro() {
    let Some(install) = install() else {
        eprintln!("SKIP: no pinned LOK build");
        return;
    };
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut worker = Worker::spawn(&install, &dir.path().join("profile")).expect("the worker");
    let refused = worker.run_macro(MACRO_URL);
    assert!(
        refused.is_err(),
        "LOK-11: the production worker keeps the gate"
    );
}

