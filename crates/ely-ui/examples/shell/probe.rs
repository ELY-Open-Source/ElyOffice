mod support;

use std::time::{Duration, Instant};

use ely_gpui_component::theme::ActiveTheme;
use ely_ui::{Root, commands};
use gpui::{App, Context, Entity, FocusHandle, Window};

use crate::Gallery;
use crate::dialogs::Overlay;
use support::{
    Probe, chord, file_menu, menu_action, plain, ran_first, secondary, shift_p, type_keys,
};

pub(crate) use support::run;

impl Gallery {
    fn ready(&mut self, condition: bool) -> bool {
        if self.frames >= 100 || condition {
            self.frames = 0;
            return true;
        }
        self.frames += 1;
        false
    }

    fn delete_ready(&mut self, window: &Window, cx: &App) -> bool {
        let away = self.away(&self.props.read(cx).anchor, window, cx);
        self.ready(away) && matches!(self.dialog, Some(Overlay::Delete))
    }

    fn away(&self, anchor: &FocusHandle, window: &Window, cx: &App) -> bool {
        window
            .focused(cx)
            .map(|focus| focus != *anchor)
            .unwrap_or(false)
    }

    fn palette_wait(
        &mut self,
        palette: bool,
        props: &FocusHandle,
        window: &Window,
        cx: &App,
    ) -> bool {
        self.ready(palette && self.away(props, window, cx))
    }

    pub(crate) fn probe_step(
        &mut self,
        root: &Entity<Root>,
        step: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Probe {
        let nav = self.nav.read(cx).anchor.clone();
        let props = self.props.read(cx).anchor.clone();
        let anchor = self.content.read(cx).anchor.clone();
        let Some(anchor) = anchor else {
            panic!("the content anchor renders before the probe runs");
        };
        if let Some(until) = self.until {
            if Instant::now() < until {
                return Probe::Again;
            }
            self.until = None;
        }
        let bounds = *self.geometry.borrow();
        let layout = |both: bool| match bounds {
            Some(area) => {
                let side = cx.theme().sidebar_width(false).to_pixels(window.rem_size());
                let sheet = cx.theme().sheet_size().to_pixels(window.rem_size());
                let width = window.viewport_size().width;
                let panels = if both { side + sheet } else { side };
                crate::close(area.origin.x, side) && crate::close(area.size.width, width - panels)
            }
            None => false,
        };
        let palette = root.read(cx).palette_open();
        let which = |window: &Window| {
            if nav.is_focused(window) {
                "nav"
            } else if anchor.is_focused(window) {
                "content"
            } else if props.is_focused(window) {
                "props"
            } else if self.chip.is_focused(window) {
                "chip"
            } else {
                "other"
            }
        };
        match step {
            _ if std::env::var("ELY_PROBE_HOLD_AT")
                .ok()
                .and_then(|v| v.parse().ok())
                == Some(step) =>
            {
                if self.frames == 0 {
                    log::info!("probe: hold at {step}");
                    self.frames = 1;
                }
                return Probe::Again;
            }
            0 => window.focus(&nav, cx),
            1 => {
                log::info!("probe: nav on page one shown={}", nav.is_focused(window));
                log::info!("probe: layout with both panels fits={}", layout(true));
            }
            2 => return Probe::Keys(vec![plain("tab")]),
            3 => log::info!("probe: tab 1 -> {}", which(window)),
            4 => return Probe::Keys(vec![plain("tab")]),
            5 => {
                log::info!("probe: tab 2 -> {}", which(window));
                window.focus(&props, cx);
            }
            6 => {
                log::info!(
                    "probe: props on page one shown={}",
                    props.is_focused(window)
                );
                self.page = "two".into();
                self.until = Some(Instant::now() + Duration::from_millis(400));
                cx.notify();
            }
            7 => {
                log::info!(
                    "probe: props on page two hidden={}",
                    !props.is_focused(window)
                );
                log::info!("probe: layout without right panel fits={}", layout(false));
                self.probe_focused = Some(anchor.clone());
                window.focus(&anchor, cx);
            }
            8 => {
                log::info!(
                    "probe: content on page two shown={}",
                    anchor.is_focused(window)
                );
                self.page = "one".into();
                self.until = Some(Instant::now() + Duration::from_millis(400));
                cx.notify();
            }
            9 => {
                let kept = self.probe_focused.clone();
                let Some(kept) = kept else {
                    panic!("step 7 focused the content anchor");
                };
                log::info!("probe: content focus kept={}", kept.is_focused(window));
                window.focus(&props, cx);
            }
            10 => {
                log::info!(
                    "probe: props back on page one shown={}",
                    props.is_focused(window)
                );
                log::info!("probe: layout restored fits={}", layout(true));
                let items = file_menu(cx);
                log::info!("probe: menus app={} File={}", items.0, items.1.join("|"));
                window.resize(gpui::size(gpui::px(800.0), gpui::px(600.0)));
                self.until = Some(Instant::now() + Duration::from_millis(400));
            }
            11 => {
                let viewport = window.viewport_size();
                log::info!(
                    "probe: resized viewport={}x{}",
                    f32::from(viewport.width) as u32,
                    f32::from(viewport.height) as u32
                );
                log::info!("probe: resized layout fits={}", layout(true));
                if let Some(area) = bounds {
                    let x: f32 = (area.origin.x + area.size.width / 2.0).into();
                    let y: f32 = (area.origin.y + area.size.height / 2.0).into();
                    return Probe::Click { x, y };
                }
            }
            12 => {
                log::info!("probe: resized click hits={}", anchor.is_focused(window));
                window.resize(gpui::size(gpui::px(1100.0), gpui::px(720.0)));
                self.until = Some(Instant::now() + Duration::from_millis(400));
            }
            13 => {
                let restored = window.viewport_size();
                log::info!(
                    "probe: restored viewport={}x{}",
                    f32::from(restored.width) as u32,
                    f32::from(restored.height) as u32
                );
                log::info!("probe: layout back fits={}", layout(true));
                log::info!("probe: scale factor={}", window.scale_factor());
                let map: Vec<String> = file_menu(cx)
                    .2
                    .into_iter()
                    .map(|(name, action)| format!("{name}={action}"))
                    .collect();
                log::info!("probe: menu map {}", map.join("|"));
            }
            step @ 14..=37 => {
                if step == 14 {
                    log::info!("probe: layout back fits={}", layout(true));
                }
                let base = if step < 26 { 14 } else { 26 };
                let i = (step - base) / 2;
                if (step - base) % 2 == 0 {
                    if base == 14 {
                        return Probe::Keys(vec![chord(i, cx)]);
                    }
                    return Probe::Actions(vec![menu_action(i, cx)]);
                }
                log::info!(
                    "probe: trigger {} {} -> {}",
                    if base == 14 { "keys" } else { "menu" },
                    commands::FILE[i].id,
                    ran_first(root, cx)
                );
            }
            38 => return Probe::Keys(vec![shift_p(cx)]),
            39 => {
                if !self.palette_wait(palette, &props, window, cx) {
                    return Probe::Again;
                }
                log::info!("probe: palette open={palette}");
                log::info!(
                    "probe: palette took focus={}",
                    self.away(&props, window, cx)
                );
                return Probe::Keys(type_keys("open"));
            }
            40 => {
                log::info!("probe: palette open={palette}");
                return Probe::Keys(vec![plain("enter")]);
            }
            41 => {
                log::info!("probe: palette open={palette}");
                return Probe::Keys(vec![shift_p(cx)]);
            }
            42 => {
                if !self.palette_wait(palette, &props, window, cx) {
                    return Probe::Again;
                }
                log::info!("probe: palette open={palette}");
                return Probe::Keys(vec![shift_p(cx)]);
            }
            43 => {
                log::info!("probe: palette open={palette}");
                log::info!(
                    "probe: palette toggle-off returned focus={}",
                    anchor.is_focused(window)
                );
                return Probe::Keys(vec![shift_p(cx)]);
            }
            44 => {
                if !self.palette_wait(palette, &props, window, cx) {
                    return Probe::Again;
                }
                return Probe::Keys(vec![plain("escape")]);
            }
            45 => {
                let palette = root.read(cx).palette_open();
                log::info!("probe: esc closed palette={}", !palette);
                return Probe::Keys(vec![shift_p(cx)]);
            }
            46 => {
                if !self.palette_wait(palette, &props, window, cx) {
                    return Probe::Again;
                }
                return Probe::Keys(type_keys("save"));
            }
            47 => return Probe::Keys(vec![plain("enter")]),
            48 => {
                log::info!("probe: blocked save -> {}", ran_first(root, cx));
                self.selection = vec![0, 1];
                cx.notify();
                let rows: Vec<String> = self
                    .props_rows(cx)
                    .into_iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect();
                log::info!("probe: props rows {}", rows.join("|"));
            }
            49 => {
                self.open_rename(window, cx);
                return Probe::Keys(
                    type_keys("fail")
                        .into_iter()
                        .chain([plain("enter")])
                        .collect(),
                );
            }
            50 => {
                let Some(Overlay::Rename { field }) = &self.dialog else {
                    panic!("the rename dialog stays after a failed check");
                };
                log::info!("probe: rename open=true text={}", field.read(cx).text());
                field.update(cx, |field, cx| field.set_text("Appendix", cx));
                return Probe::Keys(vec![plain("enter")]);
            }
            51 => {
                log::info!("probe: renamed title={}", self.page_titles().join("|"));
                self.selection = vec![0];
                self.open_rename(window, cx);
            }
            52 => {
                let Some(Overlay::Rename { field }) = &self.dialog else {
                    panic!("the rename dialog opens for the CJK test");
                };
                field.update(cx, |field, cx| field.set_text("", cx));
                return Probe::Keys(type_keys("页面1"));
            }
            53 => {
                let Some(Overlay::Rename { field }) = &self.dialog else {
                    panic!("the rename dialog holds the CJK text");
                };
                log::info!("probe: cjk text={}", field.read(cx).text());
                return Probe::KeyUpOnly(plain("enter"));
            }
            54 => {
                let Some(Overlay::Rename { field }) = &self.dialog else {
                    panic!("an enter key-up alone never submits");
                };
                log::info!(
                    "probe: keyup-only enter keeps dialog=true titles={}",
                    self.page_titles().join("|")
                );
                log::info!("probe: keyup kept text={}", field.read(cx).text());
                return Probe::Keys(vec![secondary("z", cx)]);
            }
            55 => {
                let Some(Overlay::Rename { field }) = &self.dialog else {
                    panic!("the rename dialog stays through undo");
                };
                log::info!("probe: cjk undo={}", field.read(cx).text());
                self.selection = vec![1];
                cx.notify();
                return Probe::Keys(vec![plain("escape")]);
            }
            56 => {
                if self.dialog.is_none() {
                    self.open_delete(cx);
                    return Probe::Again;
                }
                if !self.delete_ready(window, cx) {
                    return Probe::Again;
                }
                return Probe::Keys(vec![plain("tab"), plain("enter")]);
            }
            57 => {
                log::info!(
                    "probe: delete cancel pages={}",
                    self.page_titles().join("|")
                );
                self.open_delete(cx);
            }
            58 => {
                if !self.delete_ready(window, cx) {
                    return Probe::Again;
                }
                return Probe::Keys(vec![plain("tab"), plain("tab"), plain("enter")]);
            }
            59 => {
                log::info!("probe: delete done pages={}", self.page_titles().join("|"));
                self.open_delete(cx);
            }
            60 => {
                if !self.delete_ready(window, cx) {
                    return Probe::Again;
                }
                return Probe::Keys(vec![plain("tab"), plain("tab"), plain("enter")]);
            }
            61 => {
                log::info!("probe: delete last pages={}", self.page_titles().join("|"));
                self.open_rename(window, cx);
                log::info!("probe: empty rename refused={}", self.dialog.is_none());
                self.open_delete(cx);
                log::info!("probe: empty delete refused={}", self.dialog.is_none());
                self.start_task(false, cx);
                return Probe::Keys(vec![shift_p(cx)]);
            }
            62 => {
                let palette = root.read(cx).palette_open();
                if !self.ready(palette && self.away(&props, window, cx)) {
                    return Probe::Again;
                }
                log::info!(
                    "probe: palette during task at {}",
                    self.task_state(cx).unwrap_or_else(|| "none".into())
                );
                return Probe::Keys(vec![plain("escape")]);
            }
            63 => {
                if !self.ready(self.task_state(cx).as_deref() == Some("Succeeded")) {
                    return Probe::Again;
                }
                log::info!(
                    "probe: task export -> {}",
                    self.task_state(cx).unwrap_or_else(|| "none".into())
                );
                self.start_task(false, cx);
            }
            64 => {
                if !self.ready(self.beats_began() >= 1) {
                    return Probe::Again;
                }
                let chip = self.chip.clone();
                window.focus(&chip, cx);
                return Probe::Keys(vec![plain("enter")]);
            }
            65 => {
                if !self.ready(self.task_state(cx).as_deref() == Some("Cancelled")) {
                    return Probe::Again;
                }
                log::info!("probe: task cancel beats={}", self.beats_began());
                self.start_task(true, cx);
            }
            66 => {
                if !self.ready(self.beats_began() == 3) {
                    return Probe::Again;
                }
                if std::env::var("ELY_PROBE_HOLD_AT").as_deref() == Ok("running") {
                    log::info!("probe: hold at running");
                    return Probe::Again;
                }
                let chip = self.chip.clone();
                window.focus(&chip, cx);
                return Probe::Keys(vec![plain("enter")]);
            }
            67 => {
                if !self.ready(matches!(self.task_state(cx).as_deref(), Some(state) if state.starts_with("Failed("))) {
                    return Probe::Again;
                }
                if std::env::var("ELY_PROBE_HOLD_AT").as_deref() == Ok("failed") {
                    log::info!("probe: hold at failed");
                    return Probe::Again;
                }
                log::info!(
                    "probe: task error kept -> {}",
                    self.task_state(cx).unwrap_or_else(|| "none".into())
                );
                window.focus(&nav, cx);
                return Probe::Keys(vec![plain("tab")]);
            }
            68 => {
                if self.chip.is_focused(window) {
                    log::info!("probe: chip by tab=true tries={}", self.frames);
                    self.frames = 0;
                    return Probe::Keys(vec![plain("enter")]);
                }
                if self.frames >= 12 {
                    log::info!("probe: chip by tab=false");
                    self.frames = 0;
                    return Probe::Keys(vec![plain("enter")]);
                }
                self.frames += 1;
                return Probe::KeysAgain(vec![plain("tab")]);
            }
            69 => {
                if !self.ready(self.task_state(cx).as_deref() == Some("Succeeded")) {
                    return Probe::Again;
                }
                log::info!(
                    "probe: task after retry -> {}",
                    self.task_state(cx).unwrap_or_else(|| "none".into())
                );
            }
            70 => {
                return Probe::Keys(vec![shift_p(cx)]);
            }
            71 => {
                if !self.palette_wait(palette, &props, window, cx) {
                    return Probe::Again;
                }
                return Probe::KeyUpOnly(plain("enter"));
            }
            72 => {
                log::info!(
                    "probe: keyup-only enter keeps palette={} titles={}",
                    root.read(cx).palette_open(),
                    self.page_titles().join("|")
                );
                return Probe::Keys(vec![plain("escape")]);
            }
            73 => {
                log::info!(
                    "probe: keyup palette closed={}",
                    !root.read(cx).palette_open()
                );
            }
            _ => {
                let root = root.read(cx);
                log::info!("probe: palette open={}", root.palette_open());
                log::info!("probe: recent={}", root.recent().join(","));
                return Probe::Done;
            }
        }
        Probe::Next
    }
}
