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
    let updates = cx.new(|_| made(Installed::Package(Package::Deb), None));
    updates.update(cx, |updates, cx| {
        updates.checked(Ok(Check::Available(release("9.1.0", "https://c"))), false, cx);
        assert_eq!(updates.command().as_deref(), Some("sudo apt install ./ely-tree_9.1.0-1_amd64.deb"));
        updates.download(cx);
        assert_eq!(updates.offer.as_ref().map(|(_, phase)| phase.clone()), Some(Phase::Offered));
    });
    let bundled = cx.new(|_| made(bundle(), Some((release("9.1.0", "https://c"), Phase::Offered))));
    assert_eq!(bundled.read_with(cx, |updates, _| updates.command()), None);
}

#[test]
fn the_manifest_is_the_latest_releases_file() {
    let want = format!("https://github.com/ELY-Open-Source/ElyOffice/releases/latest/download/ely-doc-{}.json", update::platform());
    assert_eq!(latest(AppId::Doc), want);
}
