use anyhow::{Result, bail};
use ely_gpui_component::theme::{Mix, Mode, Palette, Syntax, contrast, over};
use gpui::{Hsla, hsla};

use super::colors::{Colors, HUE_NAMES};

const TEXT: f32 = 4.5;
const MARK: f32 = 3.0;
const BLACK: Hsla = hsla(0.0, 0.0, 0.0, 1.0);
const WHITE: Hsla = hsla(0.0, 0.0, 1.0, 1.0);

const SURFACE: f32 = 0.04;
const OVERLAY: f32 = 0.06;
const SUNKEN: f32 = 0.08;
const HOVER: f32 = 0.08;
const ACTIVE: f32 = 0.14;
const SUBTLE: f32 = 0.16;

const SURFACES: &[&str] = &["bg", "surface", "sunken", "overlay", "hover", "active"];

/// Mirrors the library's A11yChecker: text colors over their surfaces.
const TEXT_PAIRS: &[(&str, &[&str])] = &[
    ("fg", SURFACES),
    ("fg_muted", SURFACES),
    ("fg_subtle", SURFACES),
    ("link", &["bg", "surface", "sunken"]),
    ("on_accent", &["accent", "accent_hover"]),
    ("tooltip_fg", &["tooltip_bg"]),
    ("success", &["bg", "success_subtle"]),
    ("warning", &["bg", "warning_subtle"]),
    ("danger", &["bg", "danger_subtle"]),
    ("info", &["bg", "info_subtle"]),
];

const CODE_SURFACES: [&str; 3] = ["surface", "sunken", "hover"];

/// `color` via `via` to `extreme`, until it reads `need`:1.
fn lift(color: Hsla, grounds: &[Hsla], via: Hsla, extreme: Hsla, need: f32) -> Hsla {
    let toward = |from: Hsla, to: Hsla, from_step: u32| {
        (from_step..=20).map(move |step| from.mix(&to, step as f32 / 20.0))
    };
    toward(color, via, 0)
        .chain(toward(via, extreme, 1))
        .find(|candidate| grounds.iter().all(|g| contrast(*candidate, *g) >= need))
        .unwrap_or(extreme)
}

/// The palette, and the hues Ely's colors replace.
pub(super) fn build(c: &Colors) -> Result<(Palette, Vec<&'static str>)> {
    let mut p = match c.mode {
        Mode::Dark => Palette::dark(false),
        Mode::Light => Palette::light(false),
    };
    let bg = c.bg;
    let tint = |t: f32| bg.mix(&c.fg, t);
    let grounds = [
        bg,
        tint(SURFACE),
        tint(OVERLAY),
        tint(SUNKEN),
        tint(HOVER),
        tint(ACTIVE),
    ];
    let worst = |color: Hsla| {
        grounds
            .iter()
            .map(|g| contrast(color, *g))
            .fold(f32::MAX, f32::min)
    };
    let extreme = if worst(WHITE) > worst(BLACK) {
        WHITE
    } else {
        BLACK
    };
    let fg = lift(c.fg, &grounds, extreme, extreme, TEXT);
    let lifted = |color: Hsla, grounds: &[Hsla], need: f32| lift(color, grounds, fg, extreme, need);
    let text = |color: Hsla| lifted(color, &grounds, TEXT);

    let mut standing_in = Vec::new();
    let mut hue = |i: usize, ely: Hsla| {
        c.hues[i].unwrap_or_else(|| {
            standing_in.push(HUE_NAMES[i]);
            ely
        })
    };
    let (red, green, yellow, blue) = (
        hue(0, p.danger),
        hue(1, p.success),
        hue(2, p.warning),
        hue(3, p.info),
    );
    let (magenta, cyan) = (hue(4, p.chart[4]), hue(5, p.chart[1]));
    let orange = c.orange.unwrap_or(yellow);
    let accent = c.accent.or(c.hues[3]).unwrap_or_else(|| {
        standing_in.push("accent");
        p.accent
    });
    let bright = |i: usize, hue: Hsla| c.bright[i].unwrap_or(hue);

    p.bg = bg;
    (p.surface, p.overlay, p.sunken, p.hover, p.active) =
        (grounds[1], grounds[2], grounds[3], grounds[4], grounds[5]);
    p.border = tint(0.16);
    p.border_strong = tint(0.32);
    p.fg = fg;
    p.fg_muted = text(fg.mix(&bg, 0.25));
    p.fg_subtle = text(fg.mix(&bg, 0.45));
    p.fg_disabled = fg.mix(&bg, 0.65);
    p.accent = accent;
    p.on_accent = if contrast(WHITE, accent) > contrast(BLACK, accent) {
        WHITE
    } else {
        BLACK
    };
    let away = if p.on_accent == WHITE { BLACK } else { WHITE };
    p.accent_hover = [accent.mix(&fg, 0.12), accent.mix(&away, 0.12)]
        .into_iter()
        .find(|hover| contrast(p.on_accent, *hover) >= TEXT)
        .unwrap_or(accent);
    p.focus = lifted(accent, &grounds[..2], MARK);
    p.link = text(accent);
    p.selection = c.selection;
    p.success_subtle = bg.mix(&green, SUBTLE);
    p.warning_subtle = bg.mix(&yellow, SUBTLE);
    p.danger_subtle = bg.mix(&red, SUBTLE);
    p.info_subtle = bg.mix(&blue, SUBTLE);
    p.success = lifted(green, &[bg, p.success_subtle], TEXT);
    p.warning = lifted(yellow, &[bg, p.warning_subtle], TEXT);
    p.danger = lifted(red, &[bg, p.danger_subtle], TEXT);
    p.info = lifted(blue, &[bg, p.info_subtle], TEXT);
    p.tooltip_bg = fg;
    p.tooltip_fg = bg;
    p.chart = [
        blue,
        cyan,
        yellow,
        red,
        magenta,
        green,
        orange,
        bright(5, cyan),
    ]
    .map(|hue| lifted(hue, &[bg], MARK));
    p.ansi = [
        bg,
        red,
        green,
        yellow,
        blue,
        magenta,
        cyan,
        fg,
        p.fg_subtle,
        bright(0, red),
        bright(1, green),
        bright(2, yellow),
        bright(3, blue),
        bright(4, magenta),
        bright(5, cyan),
        c.bright_fg,
    ];
    let s = &mut p.syntax;
    s.keyword = text(magenta);
    s.string = text(green);
    s.number = text(orange);
    s.comment = p.fg_subtle;
    s.function = text(blue);
    s.type_name = text(yellow);
    s.constant = text(orange);
    s.property = text(cyan);
    s.tag = text(red);
    s.attribute = text(yellow);
    s.operator = text(cyan);
    s.punctuation = p.fg_muted;
    s.variable = fg;
    verify(&p)?;
    Ok((p, standing_in))
}

/// Fails on the first pair the library's audit fails.
fn verify(p: &Palette) -> Result<()> {
    let check = |front: &str, color: Hsla, back: &str, need: f32| -> Result<()> {
        let ground = p.token(back);
        let ratio = contrast(over(color, ground), ground);
        if ratio < need {
            bail!("{front} on {back} reads {ratio:.2}:1, needs {need}:1");
        }
        Ok(())
    };
    for (front, backs) in TEXT_PAIRS {
        for back in *backs {
            check(front, p.token(front), back, TEXT)?;
        }
    }
    for name in Syntax::NAMES {
        for back in CODE_SURFACES {
            check(&format!("syntax.{name}"), p.syntax.token(name), back, TEXT)?;
        }
    }
    for back in ["bg", "surface"] {
        check("focus", p.focus, back, MARK)?;
    }
    for (ix, hue) in p.chart.iter().enumerate() {
        check(&format!("chart hue {ix}"), *hue, "bg", MARK)?;
    }
    Ok(())
}
