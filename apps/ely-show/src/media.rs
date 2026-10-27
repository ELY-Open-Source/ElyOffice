//! Slide audio and video: insert, play, explain failures (SHOW-10).

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use ely_gpui_component::{
    buttons::Button, data_display::PropertyGroup, feedback::InlineMessage, primitives::Severity,
    theme::ControlSize,
};
use ely_lok::{LokView, show::Media};
use ely_office::prelude::*;
use ely_gpui_component::primitives::Retired;
use ely_platform::media::{Frame, Player, State};
use gpui::{AnyElement, App, Context, Entity, ParentElement as _, Styled as _, TaskExt as _, Window, div};

use crate::deck::Deck;
use crate::design::{Panel, on_deck};

/// How long a file may take to show it plays.
const PROBE: Duration = Duration::from_secs(10);
/// A sound's frame on the slide, 1/100 mm.
const SOUND: (i64, i64) = (2000, 2000);

/// A file checked before it goes in.
pub struct Probe {
    tab: u64,
    path: PathBuf,
    player: Player,
    started: Instant,
}

/// The media object playing: tab, slide, place, URL.
pub struct Playing {
    tab: u64,
    slide: String,
    index: usize,
    url: String,
    player: Player,
    frame: Option<Picture>,
}

/// A video picture as this platform draws it.
#[cfg(target_os = "macos")]
pub(crate) type Picture = gpui::SurfaceSource;
#[cfg(not(target_os = "macos"))]
pub(crate) type Picture = std::sync::Arc<gpui::RenderImage>;

/// The player's frame, ready to draw.
pub(crate) fn picture(frame: Frame) -> Picture {
    #[cfg(target_os = "macos")]
    return frame.into();
    #[cfg(any(windows, target_os = "linux"))]
    return crate::preview::picture(frame.size, frame.bgra);
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    match frame {}
}

/// A picture no longer drawn, for `retired` to free.
pub(crate) fn retire(retired: &mut Retired, picture: Picture) {
    #[cfg(target_os = "macos")]
    let _ = (retired, picture);
    #[cfg(not(target_os = "macos"))]
    retired.push(picture);
}

/// `m:ss`.
fn clock(seconds: f64) -> String {
    let whole = seconds.max(0.0).round() as u64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

fn name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

impl Deck {
    /// Picks a sound or video, checking it plays first.
    pub fn pick_media(&mut self, cx: &mut Context<Self>) {
        self.pick(false, cx, |deck, path, cx| deck.probe_media(path, cx));
    }

    fn probe_media(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(tab) = self.core.active else {
            return;
        };
        let player = match Player::open(&path) {
            Ok(player) => player,
            Err(error) => return self.media_refused(&path, &error.to_string(), cx),
        };
        log::info!("show: checks a media file");
        let started = Instant::now();
        self.probe = Some(Probe {
            tab,
            path,
            player,
            started,
        });
        cx.spawn(async move |this, cx| -> gpui::Result<()> {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                if this.update(cx, |deck, cx| deck.probe_tick(cx))? {
                    return Ok(());
                }
            }
        })
        .detach_and_log_err(cx);
    }

    /// One look at the file being checked; true when settled.
    fn probe_tick(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(probe) = &self.probe else {
            return true;
        };
        let state = probe.player.state();
        if state == State::Loading && probe.started.elapsed() < PROBE {
            return false;
        }
        let probe = self.probe.take().expect("checked above");
        match state {
            State::Ready { video, .. } => self.insert_media(&probe, video, cx),
            State::Failed(reason) => self.media_refused(&probe.path, &reason, cx),
            State::Loading => self.media_refused(&probe.path, "it did not load in time", cx),
        }
        true
    }

    fn insert_media(&mut self, probe: &Probe, video: Option<(u32, u32)>, cx: &mut Context<Self>) {
        let Some(tab) = self.tab_mut(probe.tab) else {
            return;
        };
        let slide = tab.extra.shown_size();
        // Pixels at 96 per inch, within most of the slide.
        let (w, h) = match video {
            Some((w, h)) => {
                let (w, h) = (i64::from(w) * 2540 / 96, i64::from(h) * 2540 / 96);
                let fit = (slide.0 as f64 * 0.8 / w as f64)
                    .min(slide.1 as f64 * 0.8 / h as f64)
                    .min(1.0);
                ((w as f64 * fit) as i64, (h as f64 * fit) as i64)
            }
            None => SOUND,
        };
        let url = ely_lok::file_url(&probe.path);
        log::info!("show: inserts media {w} x {h}");
        self.run_in(
            Some(probe.tab),
            "media.insert",
            &[&url, "false", &w.to_string(), &h.to_string()],
            cx,
        );
        self.force_reread(probe.tab, cx);
    }

    fn media_refused(&mut self, path: &Path, reason: &str, cx: &mut Context<Self>) {
        log::warn!("show: media refused: {reason}");
        let text = Deck::text(
            cx,
            "show.media.refused",
            &[("name", &name(path)), ("reason", reason)],
        );
        self.toast(text, Severity::Danger, cx);
    }

    /// The shown slide's name and its media at `index`.
    fn here(&self, index: usize) -> Option<(u64, String, Media)> {
        let tab = self.active_tab()?;
        let slide = tab
            .extra
            .slides
            .list
            .get(tab.extra.slides.current)?
            .name
            .clone();
        let media = tab.extra.media.iter().find(|media| media.index == index)?;
        Some((tab.id, slide, media.clone()))
    }

    /// Stops playback unless the same object is still shown.
    pub(crate) fn check_playing(&mut self) {
        let Some(playing) = &self.playing else {
            return;
        };
        let same = self.here(playing.index).is_some_and(|(tab, slide, media)| {
            (tab, slide, media.url) == (playing.tab, playing.slide.clone(), playing.url.clone())
        });
        if !same {
            log::info!("show: the playing media object is gone from view");
            playing.player.pause();
            self.drop_playing();
        }
    }

    /// Ends playback; its last picture is freed.
    fn drop_playing(&mut self) {
        if let Some(picture) = self.playing.take().and_then(|playing| playing.frame) {
            retire(&mut self.retired, picture);
        }
    }

    /// The media object picked on the shown slide.
    pub(crate) fn picked_media(&self) -> Option<(u64, Media)> {
        let tab = self.active_tab()?;
        let objects = tab.extra.objects.as_ref()?;
        let mut picked = objects
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.selected);
        let (at, _) = picked.next().filter(|_| picked.next().is_none())?;
        let media = tab.extra.media.iter().find(|media| media.index == at)?;
        Some((tab.id, media.clone()))
    }

    /// Plays or pauses the picked media object.
    pub fn toggle_media(&mut self, cx: &mut Context<Self>) {
        let Some((_, media)) = self.picked_media() else {
            return;
        };
        let Some((tab, slide, _)) = self.here(media.index) else {
            return;
        };
        self.check_playing();
        if let Some(playing) = self.playing.as_ref().filter(|p| p.index == media.index) {
            match (playing.player.playing(), playing.player.ended()) {
                (true, _) => playing.player.pause(),
                // A finished clip starts over.
                (false, true) => {
                    playing.player.seek(0.0);
                    playing.player.play();
                }
                (false, false) => playing.player.play(),
            }
            return cx.notify();
        }
        let opened = (ely_lok::file_path(&media.file).map_err(|error| error.to_string()))
            .and_then(|path| Player::open(&path).map_err(|error| error.to_string()));
        match opened {
            Ok(player) => {
                log::info!("show: plays media object {}", media.index);
                player.play();
                self.drop_playing();
                self.playing = Some(Playing {
                    tab,
                    slide,
                    index: media.index,
                    url: media.url.clone(),
                    player,
                    frame: None,
                });
            }
            Err(error) => self.media_refused(Path::new(&media.url), &error, cx),
        }
        cx.notify();
    }

    /// The playing video over its object; frames while it runs.
    pub(crate) fn media_layer(
        &mut self,
        view: &Entity<LokView>,
        window: &mut Window,
        cx: &App,
    ) -> Option<AnyElement> {
        self.check_playing();
        let index = self.playing.as_ref()?.index;
        let tab = self.active_tab()?;
        let rect = tab.extra.objects.as_ref()?.objects.get(index)?.rect;
        let repeat = tab.extra.media.iter().any(|m| m.index == index && m.repeat);
        let playing = self.playing.as_mut()?;
        // A failure shows in the panel, from the player's state.
        let drawing = matches!(playing.player.tick(repeat), Ok(true));
        if let Some(frame) = playing.player.frame()
            && let Some(old) = playing.frame.replace(picture(frame))
        {
            retire(&mut self.retired, old);
        }
        if drawing {
            window.request_animation_frame();
        }
        let frame = playing.frame.clone()?;
        let twips = |mm100: i64| (mm100 * 72 / 127) as i32;
        let (x, y, w, h) = rect;
        let lok = view.read(cx);
        let (top_left, bottom_right) = (
            lok.place((twips(x), twips(y))),
            lok.place((twips(x + w), twips(y + h))),
        );
        let bounds = (top_left, bottom_right - top_left);
        Some(video(frame, bounds))
    }
}

/// A video picture over its object.
pub(crate) fn video(frame: Picture, (at, size): (gpui::Point<gpui::Pixels>, gpui::Point<gpui::Pixels>)) -> AnyElement {
    use gpui::IntoElement as _;
    #[cfg(target_os = "macos")]
    let shown = gpui::surface(frame);
    #[cfg(not(target_os = "macos"))]
    let shown = gpui::StyledImage::object_fit(gpui::img(gpui::ImageSource::Render(frame)), gpui::ObjectFit::Fill);
    shown.absolute().left(at.x).top(at.y).w(size.x).h(size.y).into_any_element()
}

impl Panel {
    /// Play, where it is, and why it cannot play.
    pub(crate) fn media_group(&self, cx: &mut Context<Self>) -> Option<PropertyGroup> {
        let deck = self.deck.upgrade()?;
        let (tab, media) = deck.read(cx).picked_media()?;
        let label = |key: &str| Deck::text(cx, key, &[]);
        let group = PropertyGroup::new(label("show.media.group"));
        if media.missing {
            let what = ely_lok::file_path(&media.url)
                .map_or(media.url.clone(), |path| path.display().to_string());
            let why = Deck::text(cx, "show.media.missing", &[("path", &what)]);
            return Some(group.row(
                label("show.media.file"),
                InlineMessage::new(Severity::Danger, why),
            ));
        }
        let read = deck.read(cx);
        let playing = read
            .playing
            .as_ref()
            .filter(|p| (p.tab, p.index) == (tab, media.index));
        let (running, state) = match playing {
            Some(p) => (
                p.player.playing(),
                Some((p.player.state(), p.player.position())),
            ),
            None => (false, None),
        };
        if let Some((State::Failed(reason), _)) = &state {
            let why = Deck::text(cx, "show.media.failed", &[("reason", reason)]);
            return Some(group.row(
                label("show.media.file"),
                InlineMessage::new(Severity::Danger, why),
            ));
        }
        let weak = self.deck.clone();
        let toggle = Button::new(
            "show-media-play",
            label(if running {
                "show.media.pause"
            } else {
                "show.media.play"
            }),
        )
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| on_deck(&weak, cx, |deck, cx| deck.toggle_media(cx)));
        let time = match state {
            Some((State::Ready { seconds, .. }, at)) => {
                format!("{} / {}", clock(at), clock(seconds))
            }
            _ => clock(0.0),
        };
        let row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(toggle)
            .child(time);
        Some(group.row(label("show.media.play"), row))
    }
}
