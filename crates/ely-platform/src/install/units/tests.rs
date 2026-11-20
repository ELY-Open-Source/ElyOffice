use std::{cell::Cell, path::Path};

use super::{SHADOW, appimage_written, unshadow};

/// The units AppRun's test reads too: `appimage-*` alone go.
fn samples() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/engine-units");
    let mut found: Vec<_> = std::fs::read_dir(&dir)
        .expect("the sample units")
        .map(|entry| {
            let path = entry.expect("an entry").path();
            let name = path.file_stem().expect("a name").to_string_lossy().into_owned();
            (name, std::fs::read_to_string(&path).expect("a sample"))
        })
        .collect();
    found.sort();
    found
}

/// A fake home under the data root; pr.yml sets none.
fn home() -> tempfile::TempDir {
    match std::env::var_os("ELY_DATA_ROOT") {
        Some(root) => {
            std::fs::create_dir_all(&root).expect("the data root");
            tempfile::tempdir_in(root).expect("a fake home")
        }
        None => tempfile::tempdir().expect("a fake home"),
    }
}

#[test]
fn only_an_appimage_s_unit_goes_with_its_links_and_one_reload() {
    let samples = samples();
    assert_eq!(samples.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(), ["appimage-0.1.16", "hand", "hand-env", "package"]);
    for (name, text) in samples {
        let home = home();
        let units = home.path().join(".config/systemd/user");
        let wants = units.join("graphical-session.target.wants");
        std::fs::create_dir_all(&wants).expect("the wants folder");
        let shadow = units.join(SHADOW);
        std::fs::write(&shadow, &text).expect("the unit");
        for slug in ["ely-doc", "ely-table", "ely-show"] {
            std::os::unix::fs::symlink(&shadow, wants.join(format!("ely-engine@{slug}.service"))).expect("a link");
        }
        std::os::unix::fs::symlink("/elsewhere", wants.join("other.service")).expect("another link");
        let reloads = Cell::new(0);
        let removed = unshadow(&units, || {
            reloads.set(reloads.get() + 1);
            Ok(())
        })
        .expect("unshadow");
        let goes = name.starts_with("appimage-");
        assert_eq!(appimage_written(&text), goes, "{name}");
        assert_eq!((removed.len(), reloads.get()), if goes { (4, 1) } else { (0, 0) }, "{name}");
        assert_eq!(shadow.exists(), !goes, "{name}");
        let left: Vec<_> = std::fs::read_dir(&wants).expect("wants").map(|entry| entry.expect("entry").file_name()).collect();
        assert_eq!(left.len(), if goes { 1 } else { 4 }, "{name}: {left:?}");
    }
}

#[test]
fn no_unit_means_no_reload() {
    let home = home();
    let units = home.path().join(".config/systemd/user");
    let reloads = Cell::new(0);
    let removed = unshadow(&units, || {
        reloads.set(1);
        Ok(())
    })
    .expect("nothing to do");
    assert!(removed.is_empty() && reloads.get() == 0);
}

#[test]
fn a_failed_reload_surfaces() {
    let home = home();
    let units = home.path().join(".config/systemd/user");
    std::fs::create_dir_all(&units).expect("units");
    let (_, text) = samples().into_iter().find(|(name, _)| name.starts_with("appimage-")).expect("a sample");
    std::fs::write(units.join(SHADOW), text).expect("the unit");
    let error = unshadow(&units, || Err(std::io::Error::other("no user manager"))).expect_err("the reload failed");
    assert_eq!(error.to_string(), "no user manager");
}
