use super::*;
use ely_platform::install::Package;

fn release(version: &str, url: &str) -> Release {
    Release {
        version: semver::Version::parse(version).expect("a version"),
        url: url.into(),
        sha256: String::new(),
    }
}

fn made(installed: Installed, offer: Option<(Release, Phase)>) -> Updates {
    Updates {
        app: AppId::Tree,
        installed,
        offer,
        dismissed: false,
        pending: None,
        checked: None,
        note: None,
        dialog: None,
    }
}

fn bundle() -> Installed {
    Installed::Bundle(PathBuf::from("/Applications/ElyTree.app"))
}

#[gpui::test]
fn a_check_brings_back_a_dismissed_download(cx: &mut gpui::TestAppContext) {
    let fetched = Some((release("9.0.0", "https://b"), Phase::Fetched(PathBuf::from("B.dmg"))));
    let updates = cx.new(|_| Updates { dismissed: true, ..made(bundle(), fetched) });
    updates.update(cx, |updates, cx| updates.check(true, cx));
    updates.read_with(cx, |updates, _| {
        assert!(!updates.dismissed, "the bar shows again");
        assert_eq!(
            updates.offer.as_ref().map(|(_, phase)| phase.clone()),
            Some(Phase::Fetched(PathBuf::from("B.dmg")))
        );
    });
}

#[gpui::test]
fn a_download_for_an_older_offer_is_dropped(cx: &mut gpui::TestAppContext) {
    let fetching = Some((release("9.0.0", "https://b"), Phase::Fetching(0.5)));
    let updates = cx.new(|_| made(bundle(), fetching));
    updates.update(cx, |updates, cx| {
        updates.fetched(&release("8.0.0", "https://a"), Ok(PathBuf::from("A.dmg")), cx);
        assert_eq!(updates.offer.as_ref().map(|(_, phase)| phase.clone()), Some(Phase::Fetching(0.5)));
        updates.fetched(&release("9.0.0", "https://b"), Ok(PathBuf::from("B.dmg")), cx);
        assert_eq!(
            updates.offer.as_ref().map(|(_, phase)| phase.clone()),
            Some(Phase::Fetched(PathBuf::from("B.dmg")))
        );
    });
}

#[gpui::test]
fn a_check_keeps_its_outcome_for_the_settings(cx: &mut gpui::TestAppContext) {
    let updates = cx.new(|_| made(bundle(), None));
    updates.update(cx, |updates, cx| {
        updates.checked(Err(anyhow::anyhow!("offline")), true, cx);
        assert_eq!(updates.checked, Some(Checked::Failed("offline".into())));
        updates.checked(Ok(Check::Current), true, cx);
        assert_eq!(updates.checked, Some(Checked::Latest));
        updates.checked(Ok(Check::Available(release("9.1.0", "https://c"))), true, cx);
        assert_eq!(updates.checked, Some(Checked::Newer(semver::Version::new(9, 1, 0))));
        assert_eq!(updates.offer.as_ref().map(|(_, phase)| phase.clone()), Some(Phase::Offered));
    });
}

#[gpui::test]
fn a_packaged_app_names_its_command_and_downloads_nothing(cx: &mut gpui::TestAppContext) {
    let updates = cx.new(|_| made(Installed::Package(Package::Deb, "ely-tree".into()), None));
    let deb = if std::env::consts::ARCH == "x86_64" { "amd64" } else { "arm64" };
    let command = format!("sudo apt install ./ely-tree_9.1.0-1_{deb}.deb");
    updates.update(cx, |updates, cx| {
        updates.checked(Ok(Check::Available(release("9.1.0", "https://c"))), false, cx);
        assert_eq!(updates.command().as_deref(), Some(command.as_str()));
        updates.download(cx);
        assert_eq!(updates.offer.as_ref().map(|(_, phase)| phase.clone()), Some(Phase::Offered));
    });
    let bundled = cx.new(|_| made(bundle(), Some((release("9.1.0", "https://c"), Phase::Offered))));
    assert_eq!(bundled.read_with(cx, |updates, _| updates.command()), None);
    let nix = cx.new(|_| made(Installed::Nix, None));
    nix.update(cx, |updates, cx| {
        updates.checked(Ok(Check::Available(release("9.1.0", "https://c"))), false, cx);
        assert_eq!(updates.checked, Some(Checked::Newer(semver::Version::new(9, 1, 0))), "the settings show it");
        assert_eq!(updates.offer, None, "Nix installs it; nothing downloads");
    });
}

#[test]
fn the_manifest_is_the_latest_file_on_github_then_on_the_relay() {
    let file = format!("releases/latest/download/ely-office-{}.json", update::platform());
    let want = ["https://github.com/ELY-Open-Source/ElyOffice", "https://office.elygpui.com"].map(|home| format!("{home}/{file}"));
    assert_eq!(latest(), want);
}

#[test]
fn only_a_github_release_file_has_an_address_on_the_relay() {
    let file = "download/v0.1.15/ElyDoc-0.1.15-arm64.dmg";
    assert_eq!(mirrored(&format!("{GITHUB}/{file}")), Some(format!("https://office.elygpui.com/releases/{file}")));
    assert_eq!(mirrored("https://example.com/releases/download/v1/a.dmg"), None);
}

#[gpui::test]
fn the_dialog_follows_the_offer_and_closes_with_it(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        ely_gpui_component::init_for_tests(cx);
        crate::i18n::install_as("en-US", cx);
    });
    let updates = cx.new(|_| made(bundle(), Some((release("9.0.0", "https://a"), Phase::Offered))));
    let open = |cx: &mut gpui::TestAppContext| {
        cx.update(|cx| view::open(&updates, cx));
        cx.run_until_parked();
    };
    let phase = |phase: Phase, cx: &mut gpui::TestAppContext| {
        updates.update(cx, |updates, cx| {
            updates.offer.as_mut().expect("an offer").1 = phase;
            cx.notify();
        });
        cx.run_until_parked();
    };
    open(cx);
    open(cx);
    assert_eq!(cx.windows().len(), 1, "a second click brings the first forward");
    let first = cx.windows()[0];
    cx.update(|cx| first.update(cx, |_, window, _| window.remove_window())).expect("the dialog");
    cx.run_until_parked();
    open(cx);
    assert_eq!(cx.windows().len(), 1, "a closed dialog opens again");
    phase(Phase::Fetching(0.5), cx);
    phase(Phase::Fetched(PathBuf::from("9.dmg")), cx);
    assert_eq!(cx.windows().len(), 1, "the dialog stays while the offer does");
    let shown = updates.read_with(cx, |updates, _| updates.dialog.as_ref().map(|(shown, _)| shown.window_id()));
    assert_eq!(shown, Some(cx.windows()[0].window_id()), "it follows the new window");
    updates.update(cx, |updates, cx| updates.later(cx));
    cx.run_until_parked();
    assert!(cx.windows().is_empty(), "a dismissed offer closes its dialog");
    assert!(updates.read_with(cx, |updates, _| updates.dialog.is_none()));
}
