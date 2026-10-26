use std::{fs, thread, time::Duration};

use ely_gpui_component::theme::{Mix, Mode, Palette, contrast};
use futures::channel::mpsc::UnboundedReceiver;

use super::{OmarchyTheme, changes, colors::hex, load, parse};
use gpui::Hsla;

const BUNDLED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/omarchy");
const GENERATED: &str = include_str!("../../../tests/fixtures/omarchy-generated/tokyo-night.toml");
const TOKYO_NIGHT: &str = include_str!("../../../tests/fixtures/omarchy/tokyo-night.toml");
const LATTE: &str = include_str!("../../../tests/fixtures/omarchy/catppuccin-latte.toml");

fn bundled() -> Vec<(String, String)> {
    let mut themes: Vec<_> = fs::read_dir(BUNDLED)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| {
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            (name, fs::read_to_string(path).unwrap())
        })
        .collect();
    themes.sort();
    themes
}

fn value_of(text: &str, key: &str) -> String {
    let line = text
        .lines()
        .find(|line| line.starts_with(&format!("{key} =")))
        .unwrap();
    line.split('"').nth(1).unwrap().to_string()
}

/// The library's contrast rules, written out apart from `verify`.
fn assert_contract(name: &str, theme: &OmarchyTheme) {
    let p = &theme.palette;
    let s = &p.syntax;
    let surfaces = [p.bg, p.surface, p.overlay, p.sunken, p.hover, p.active];
    let on = |token: &str, color: Hsla, grounds: &[Hsla], need: f32| {
        for (ix, ground) in grounds.iter().enumerate() {
            let ratio = contrast(color, *ground);
            assert!(
                ratio >= need,
                "{name}: {token} on ground {ix} reads {ratio:.2}"
            );
        }
    };
    for (token, color) in [
        ("fg", p.fg),
        ("fg_muted", p.fg_muted),
        ("fg_subtle", p.fg_subtle),
        ("link", p.link),
        ("keyword", s.keyword),
        ("string", s.string),
        ("number", s.number),
        ("comment", s.comment),
        ("function", s.function),
        ("type_name", s.type_name),
        ("constant", s.constant),
        ("property", s.property),
        ("tag", s.tag),
        ("attribute", s.attribute),
        ("operator", s.operator),
        ("punctuation", s.punctuation),
        ("variable", s.variable),
    ] {
        on(token, color, &surfaces, 4.5);
    }
    for (token, color, subtle) in [
        ("success", p.success, p.success_subtle),
        ("warning", p.warning, p.warning_subtle),
        ("danger", p.danger, p.danger_subtle),
        ("info", p.info, p.info_subtle),
    ] {
        on(token, color, &[p.bg, subtle], 4.5);
    }
    on("on_accent", p.on_accent, &[p.accent, p.accent_hover], 4.5);
    on("tooltip_fg", p.tooltip_fg, &[p.tooltip_bg], 4.5);
    on("focus", p.focus, &[p.bg, p.surface], 3.0);
    for (ix, hue) in p.chart.iter().enumerate() {
        on(&format!("chart {ix}"), *hue, &[p.bg], 3.0);
    }
}

#[test]
fn every_bundled_theme_maps_with_its_colors_and_text_that_reads_on_every_surface() {
    let themes = bundled();
    assert!(themes.len() >= 22, "only {} fixtures", themes.len());
    for (name, text) in themes {
        let theme =
            parse(&text, Some(name.clone()), false).unwrap_or_else(|e| panic!("{name}: {e:#}"));

        let mode = if value_of(&text, "mode") == "dark" {
            Mode::Dark
        } else {
            Mode::Light
        };
        assert_eq!(theme.mode, mode, "{name}");
        assert!(
            theme.standing_in.is_empty(),
            "{name}: {:?}",
            theme.standing_in
        );
        assert_eq!(
            theme.palette.bg,
            hex(&value_of(&text, "background")).unwrap(),
            "{name}"
        );
        assert_eq!(
            theme.palette.accent,
            hex(&value_of(&text, "accent")).unwrap(),
            "{name}"
        );
        assert_eq!(
            theme.palette.selection,
            hex(&value_of(&text, "selection")).unwrap(),
            "{name}"
        );
        let red = hex(&value_of(&text, "red")).unwrap();
        let reads = |ground| contrast(red, ground) >= 4.5;
        if reads(theme.palette.bg) && reads(theme.palette.danger_subtle) {
            assert_eq!(
                theme.palette.danger, red,
                "{name}: a red that reads is kept"
            );
        }
        assert_contract(&name, &theme);
    }
}

#[test]
fn a_theme_whose_text_already_reads_keeps_its_foreground() {
    let theme = parse(TOKYO_NIGHT, None, false).unwrap();

    assert_eq!(theme.palette.fg, hex("#a9b1d6").unwrap());
}

#[test]
fn a_generated_theme_with_only_colorn_keys_gets_its_hues_through_omarchys_aliases() {
    let theme = parse(GENERATED, None, false).unwrap();
    let p = &theme.palette;

    assert_eq!(theme.mode, Mode::Dark);
    assert!(theme.standing_in.is_empty(), "{:?}", theme.standing_in);
    assert_eq!(p.danger, hex("#f7768e").unwrap());
    assert_eq!(p.success, hex("#9ece6a").unwrap());
    assert_eq!(p.warning, hex("#e0af68").unwrap());
    assert_eq!(p.info, hex("#7aa2f7").unwrap());
    assert_eq!(p.chart[4], hex("#ad8ee6").unwrap());
    assert_eq!(p.chart[1], hex("#449dab").unwrap());
    assert_eq!(p.chart[7], hex("#0db9d7").unwrap());
    assert_eq!(p.ansi[9], hex("#ff7a93").unwrap());
    assert_eq!(p.ansi[15], hex("#c0caf5").unwrap());
    assert_contract("generated", &theme);
}

#[test]
fn bright_hues_and_orange_follow_omarchys_derivations_when_absent() {
    let bare = "background = \"#101010\"\nforeground = \"#e0e0e0\"\nred = \"#c04040\"\nyellow = \"#c0a040\"\n";
    let theme = parse(bare, None, false).unwrap();
    let p = &theme.palette;

    assert_eq!(
        p.ansi[9],
        hex("#c04040").unwrap().mix(&hex("#ffffff").unwrap(), 0.2)
    );
    assert_eq!(p.chart[6], hex("#c0a040").unwrap());
}

#[test]
fn a_theme_that_lacks_a_hue_is_told_apart_from_one_that_has_it() {
    let no_red = TOKYO_NIGHT.replace("red = \"#f7768e\"", "");
    let theme = parse(&no_red, None, false).unwrap();

    assert_eq!(theme.standing_in, ["red"]);
    assert_eq!(theme.palette.danger, Palette::dark(false).danger);

    let bare = parse(
        "background = \"#101010\"\nforeground = \"#e0e0e0\"",
        None,
        false,
    )
    .unwrap();
    assert_eq!(
        bare.standing_in,
        [
            "red", "green", "yellow", "blue", "magenta", "cyan", "accent"
        ]
    );
}

#[test]
fn the_reader_takes_omarchys_loose_syntax_and_skips_what_omarchy_skips() {
    let loose = "# a comment\n\"background\" = '#1a1b26'\nforeground=#a9b1d6\n  accent   =   \"#7aa2f7\"  # note\nmode = \"dark\"\nodd key! = \"#ffffff\"\nname = \"a;b\"\n[table]\n";
    let theme = parse(loose, None, false).unwrap();

    assert_eq!(theme.palette.bg, hex("#1a1b26").unwrap());
    assert_eq!(theme.palette.accent, hex("#7aa2f7").unwrap());
    assert_eq!(theme.palette.fg, hex("#a9b1d6").unwrap());
}

#[test]
fn mode_comes_from_mode_then_theme_type_then_the_light_file_then_brightness() {
    let colors =
        |extra: &str, bg: &str| format!("{extra}\nbackground = \"{bg}\"\nforeground = \"#808080\"");
    let mode = |text: String, light_file: bool| parse(&text, None, light_file).unwrap().mode;

    assert_eq!(
        mode(colors("mode = \"light\"", "#000000"), false),
        Mode::Light
    );
    assert_eq!(
        mode(colors("theme_type = \"light\"", "#000000"), false),
        Mode::Light
    );
    assert_eq!(
        mode(
            colors("mode = \"dark\"\ntheme_type = \"light\"", "#ffffff"),
            false
        ),
        Mode::Dark
    );
    assert_eq!(mode(colors("", "#000000"), true), Mode::Light);
    assert_eq!(mode(colors("mode = \"dark\"", "#ffffff"), true), Mode::Dark);
    assert_eq!(mode(colors("", "#7f8080"), false), Mode::Light);
    assert_eq!(mode(colors("", "#7f7f80"), false), Mode::Dark);
}

#[test]
fn the_cause_is_named_for_a_missing_color_a_bad_color_a_bad_mode_and_unreadable_text() {
    let say = |text: &str| format!("{:#}", parse(text, None, false).unwrap_err());
    let without = |key: &str| {
        TOKYO_NIGHT
            .lines()
            .filter(|line| !line.starts_with(key))
            .collect::<Vec<_>>()
            .join("\n")
    };

    assert!(say(&without("foreground")).contains("'foreground' is missing"));
    assert!(say(&without("background")).contains("'background' is missing"));
    assert!(
        say(&TOKYO_NIGHT.replace("#7aa2f7\"\nselection", "blue\"\nselection")).contains("'accent'")
    );
    assert!(say(&TOKYO_NIGHT.replace("#f7768e", "#+12345")).contains("'#+12345' is not #rrggbb"));
    assert!(
        say(&TOKYO_NIGHT.replace("mode = \"dark\"", "mode = \"dim\""))
            .contains("not dark or light")
    );
    assert!(say("not toml at all").contains("'background' is missing"));
    assert!(say("background = \"#777777\"\nforeground = \"#000000\"").contains("4.5"));
}

#[test]
fn load_reads_the_theme_its_name_and_its_light_marker_and_a_missing_file_is_none() {
    let current = tempfile::tempdir().unwrap();
    assert!(load(current.path()).unwrap().is_none());

    let theme_dir = current.path().join("theme");
    fs::create_dir(&theme_dir).unwrap();
    fs::write(theme_dir.join("colors.toml"), LATTE).unwrap();
    fs::write(current.path().join("theme.name"), "catppuccin-latte\n").unwrap();
    let theme = load(current.path()).unwrap().expect("a theme");
    assert_eq!(theme.name.as_deref(), Some("catppuccin-latte"));
    assert_eq!(theme.mode, Mode::Light);

    fs::write(
        theme_dir.join("colors.toml"),
        "background = \"#000000\"\nforeground = \"#ffffff\"",
    )
    .unwrap();
    assert_eq!(load(current.path()).unwrap().unwrap().mode, Mode::Dark);
    fs::write(theme_dir.join("light.mode"), "").unwrap();
    assert_eq!(load(current.path()).unwrap().unwrap().mode, Mode::Light);

    fs::write(theme_dir.join("colors.toml"), "mode = \"dark\"").unwrap();
    let error = format!("{:#}", load(current.path()).unwrap_err());
    assert!(
        error.contains("colors.toml") && error.contains("'background' is missing"),
        "{error}"
    );
}

fn event_within(changes: &mut UnboundedReceiver<()>, tries: u32) -> bool {
    (0..tries).any(|_| {
        thread::sleep(Duration::from_millis(50));
        changes.try_recv().is_ok()
    })
}

#[test]
fn changes_sees_a_theme_swap_in_the_order_omarchy_does_it() {
    let current = tempfile::tempdir().unwrap();
    let (_watcher, mut events) = changes(current.path()).unwrap();

    fs::create_dir(current.path().join("theme")).unwrap();
    assert!(event_within(&mut events, 60), "no event for a new theme");
    while events.try_recv().is_ok() {}

    fs::create_dir(current.path().join("next-theme")).unwrap();
    fs::remove_dir_all(current.path().join("theme")).unwrap();
    fs::rename(
        current.path().join("next-theme"),
        current.path().join("theme"),
    )
    .unwrap();
    fs::write(current.path().join("theme.name"), "nord\n").unwrap();
    assert!(
        event_within(&mut events, 60),
        "no event for a swapped theme"
    );
}

#[test]
fn reading_the_theme_is_no_change() {
    let current = tempfile::tempdir().unwrap();
    let theme_dir = current.path().join("theme");
    fs::create_dir(&theme_dir).unwrap();
    fs::write(theme_dir.join("colors.toml"), LATTE).unwrap();
    fs::write(current.path().join("theme.name"), "catppuccin-latte\n").unwrap();
    let (_watcher, mut events) = changes(current.path()).unwrap();

    load(current.path()).unwrap().expect("a theme");
    assert!(!event_within(&mut events, 10), "a read counted as a change");
}

#[test]
fn an_unreadable_theme_name_is_an_error_and_a_missing_one_is_none() {
    let current = tempfile::tempdir().unwrap();
    let theme_dir = current.path().join("theme");
    fs::create_dir(&theme_dir).unwrap();
    fs::write(theme_dir.join("colors.toml"), LATTE).unwrap();
    assert_eq!(load(current.path()).unwrap().unwrap().name, None);

    fs::create_dir(current.path().join("theme.name")).unwrap();
    let error = format!("{:#}", load(current.path()).unwrap_err());
    assert!(error.contains("theme.name"), "{error}");
}
