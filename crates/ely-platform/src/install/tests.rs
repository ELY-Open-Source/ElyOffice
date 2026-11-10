use super::*;

/// An rpm that lists nothing.
fn no_rpm(_: &str) -> io::Result<bool> {
    Ok(false)
}

#[test]
fn the_bundle_is_the_nearest_app_folder() {
    let exe = Path::new("/Applications/ElyDoc.app/Contents/MacOS/ely-doc");
    assert_eq!(bundle_of(exe), Some(Path::new("/Applications/ElyDoc.app")));
    assert_eq!(bundle_of(Path::new("/repo/target/debug/ely-doc")), None);
}

#[test]
fn each_system_names_its_own_install() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let exe = dir.path().join("ely-doc.exe");
    let none = |os, exe: &Path, image| installed(os, exe, image, dir.path(), "ely-doc", &no_rpm).expect("reads");
    assert_eq!(none("windows", &exe, None), None, "no setup list: a dev build");
    std::fs::write(dir.path().join(LISTED), "").expect("a list");
    assert_eq!(
        none("windows", &exe, None),
        Some(Installed::Folder { dir: dir.path().to_path_buf(), exe: exe.clone() })
    );
    let image = PathBuf::from("/home/me/Apps/ElyDoc.AppImage");
    let mount = PathBuf::from("/tmp/.mount_ElyDoc");
    let mounted = Path::new("/tmp/.mount_ElyDoc/usr/bin/ely-doc");
    let ours = Some((image.clone(), mount));
    assert_eq!(none("linux", mounted, ours), Some(Installed::AppImage(image)));
    let dev = Path::new("/home/me/ely/target/ely-doc");
    assert_eq!(none("linux", dev, None), None);
    // Started from another AppImage's terminal.
    let tree = Some(("/home/me/Apps/ElyTree.AppImage".into(), "/tmp/.mount_ElyTree".into()));
    assert_eq!(none("linux", dev, tree), None, "the environment is another app's");
    let store = Path::new("/nix/store/abc-ely-doc-0.1.9-extracted/usr/bin/ely-doc");
    let wrapped = Some(("/nix/store/abc.AppImage".into(), "/nix/store/abc-ely-doc-0.1.9-extracted".into()));
    assert_eq!(none("linux", store, wrapped), None, "Nix updates it");
    let app = Path::new("/Applications/ElyDoc.app/Contents/MacOS/ely-doc");
    assert_eq!(none("macos", app, None), Some(Installed::Bundle("/Applications/ElyDoc.app".into())));
}

#[test]
fn a_package_names_its_manager_or_fails() {
    let root = tempfile::tempdir().expect("a temp dir");
    let at = |path: &str| {
        std::fs::create_dir_all(root.path().join(path)).expect("a folder");
    };
    let exe = Path::new("/opt/ely-office/ely-doc/bin/ely-doc");
    let read = || installed("linux", exe, None, root.path(), "ely-doc", &no_rpm);
    assert!(read().is_err(), "no manager lists it");
    at("var/lib/pacman/local/ely-doc-extra-1.0-1");
    assert!(read().is_err(), "another package's name");
    at("var/lib/pacman/local/ely-doc-0.1.9-1");
    assert_eq!(read().expect("reads"), Some(Installed::Package(Package::Arch, "ely-doc".into())));
    at("var/lib/dpkg/info");
    std::fs::write(root.path().join("var/lib/dpkg/info/ely-doc.list"), "").expect("a list");
    assert_eq!(read().expect("reads"), Some(Installed::Package(Package::Deb, "ely-doc".into())));
    let rpm = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir_all(rpm.path().join("usr/lib/sysimage/rpm")).expect("an rpm db");
    let unlisted = installed("linux", exe, None, rpm.path(), "ely-doc", &no_rpm);
    assert!(unlisted.is_err(), "an rpm database that lists no ElyDoc");
    let lists = |slug: &str| Ok(slug == "ely-doc");
    let read = installed("linux", exe, None, rpm.path(), "ely-doc", &lists).expect("reads");
    assert_eq!(read, Some(Installed::Package(Package::Rpm, "ely-doc".into())));
}

#[test]
fn the_suite_s_package_owns_an_app_without_its_own() {
    let exe = Path::new("/opt/ely-office/ely-doc/bin/ely-doc");
    let suite = |manager| Some(Installed::Package(manager, "ely-office".into()));
    let arch = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir_all(arch.path().join("var/lib/pacman/local/ely-office-0.1.16-1")).expect("a folder");
    assert_eq!(installed("linux", exe, None, arch.path(), "ely-doc", &no_rpm).expect("reads"), suite(Package::Arch));
    let deb = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir_all(deb.path().join("var/lib/dpkg/info")).expect("a folder");
    std::fs::write(deb.path().join("var/lib/dpkg/info/ely-office.list"), "").expect("a list");
    assert_eq!(installed("linux", exe, None, deb.path(), "ely-doc", &no_rpm).expect("reads"), suite(Package::Deb));
    // Its own package wins where both are listed.
    std::fs::write(deb.path().join("var/lib/dpkg/info/ely-doc.list"), "").expect("a list");
    let own = installed("linux", exe, None, deb.path(), "ely-doc", &no_rpm).expect("reads");
    assert_eq!(own, Some(Installed::Package(Package::Deb, "ely-doc".into())));
    let rpm = tempfile::tempdir().expect("a temp dir");
    std::fs::create_dir_all(rpm.path().join("var/lib/rpm")).expect("an rpm db");
    let lists = |name: &str| Ok(name == "ely-office");
    assert_eq!(installed("linux", exe, None, rpm.path(), "ely-doc", &lists).expect("reads"), suite(Package::Rpm));
    let version = semver::Version::new(0, 1, 16);
    assert!(Package::Arch.command("ely-office", &version).starts_with("sudo pacman -U ./ely-office-0.1.16-1-"));
}

#[test]
fn each_manager_gets_its_own_file_and_command() {
    let version = semver::Version::new(0, 1, 10);
    let file = |package: Package, arch: &str| package.file_for("ely-doc", &version, arch);
    assert_eq!(file(Package::Deb, "x86_64"), "ely-doc_0.1.10-1_amd64.deb");
    assert_eq!(file(Package::Deb, "aarch64"), "ely-doc_0.1.10-1_arm64.deb");
    assert_eq!(file(Package::Rpm, "aarch64"), "ely-doc-0.1.10-1.aarch64.rpm");
    assert_eq!(file(Package::Arch, "aarch64"), "ely-doc-0.1.10-1-aarch64.pkg.tar.zst");
    let host = std::env::consts::ARCH;
    let deb = if host == "x86_64" { "amd64" } else { "arm64" };
    assert_eq!(Package::Deb.command("ely-doc", &version), format!("sudo apt install ./ely-doc_0.1.10-1_{deb}.deb"));
    assert_eq!(Package::Rpm.command("ely-doc", &version), format!("sudo dnf install ./ely-doc-0.1.10-1.{host}.rpm"));
    assert_eq!(Package::Arch.command("ely-doc", &version), format!("sudo pacman -U ./ely-doc-0.1.10-1-{host}.pkg.tar.zst"));
}

#[test]
fn a_swap_replaces_the_app_and_leaves_no_copy() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bundle, staged) = (dir.path().join("ElyDoc.app"), dir.path().join(".ElyDoc.app.new"));
    std::fs::create_dir_all(bundle.join("old")).expect("the old app");
    std::fs::create_dir_all(staged.join("new")).expect("the new app");
    swap(&staged, &bundle).expect("swaps");
    assert!(bundle.join("new").exists() && !bundle.join("old").exists());
    assert_eq!(std::fs::read_dir(dir.path()).expect("lists").count(), 1, "nothing else is left");
}

#[cfg(target_os = "macos")]
#[test]
fn an_old_copy_that_will_not_go_leaves_the_install_done() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let (bundle, staged) = (dir.path().join("ElyDoc.app"), dir.path().join(".ElyDoc.app.new"));
    std::fs::create_dir_all(&bundle).expect("the old app");
    std::fs::write(bundle.join("version"), "old").expect("marks it");
    std::fs::create_dir_all(&staged).expect("the new app");
    std::fs::write(staged.join("version"), "new").expect("marks it");
    run(Command::new("chflags").arg("uchg").arg(bundle.join("version"))).expect("locks a file");
    swap(&staged, &bundle).expect("installed although the old copy stays");
    assert_eq!(std::fs::read_to_string(bundle.join("version")).expect("reads"), "new");
    let old = dir.path().join(".ElyDoc.app.old");
    run(Command::new("chflags").arg("nouchg").arg(old.join("version"))).expect("unlocks");
}

#[cfg(target_os = "macos")]
#[test]
fn a_dmg_installs_over_the_app() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let source = dir.path().join("src");
    std::fs::create_dir_all(source.join("ElyDoc.app/Contents")).expect("the new app");
    std::fs::write(source.join("ElyDoc.app/Contents/version"), "new").expect("marks it");
    let dmg = dir.path().join("ElyDoc.dmg");
    run(Command::new("hdiutil").args(["create", "-quiet", "-fs", "HFS+", "-srcfolder"]).arg(&source).arg(&dmg)).expect("a dmg");
    let bundle = dir.path().join("Applications/ElyDoc.app");
    std::fs::create_dir_all(bundle.join("Contents")).expect("the old app");
    std::fs::write(bundle.join("Contents/version"), "old").expect("marks it");
    install(&dmg, &bundle).expect("installs");
    assert_eq!(std::fs::read_to_string(bundle.join("Contents/version")).expect("reads"), "new");
    let empty = dir.path().join("Applications/Other.app");
    assert!(install(&dmg, &empty).is_err(), "a dmg without that app is refused");
}
