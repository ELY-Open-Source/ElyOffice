//! Audio and video on the shown slide, by a click.

use ely_platform::media::Player;
use gpui::Context;

use super::Show;
use crate::deck::Deck;
use crate::media::{Picture, picture, retire};

/// The media object playing in the show.
pub struct Playing {
    part: usize,
    index: usize,
    player: Player,
    pub frame: Option<Picture>,
}

impl Show {
    /// The shown slide's media under a point, as shares.
    pub fn media_at(&self, (x, y): (f32, f32)) -> Option<usize> {
        let part = self.part()?;
        let (w, h) = self.slide;
        self.slides[part].media.iter().position(|media| {
            let (left, top, width, height) = media.bounds;
            let (x, y) = (x * w, y * h);
            (left as f32..(left + width) as f32).contains(&x)
                && (top as f32..(top + height) as f32).contains(&y)
        })
    }

    /// Stops the media and tells the presenter why.
    fn media_failed(&mut self, reason: &str, cx: &mut Context<Self>) {
        log::warn!("show: media failed: {reason}");
        self.drop_media();
        self.notice = Some(Deck::text(cx, "show.media.failed", &[("reason", reason)]));
        cx.notify();
    }

    /// Plays, pauses or replays media `at` on the shown slide.
    pub fn toggle_media(&mut self, at: usize, cx: &mut Context<Self>) {
        let Some(part) = self.part() else {
            return;
        };
        if let Some(playing) = self.media.as_ref().filter(|p| (p.part, p.index) == (part, at)) {
            match (playing.player.playing(), playing.player.ended()) {
                (true, _) => playing.player.pause(),
                (false, true) => {
                    playing.player.seek(0.0);
                    playing.player.play();
                }
                (false, false) => playing.player.play(),
            }
            return cx.notify();
        }
        let media = &self.slides[part].media[at];
        let opened = ely_lok::file_path(&media.file)
            .map_err(|error| error.to_string())
            .and_then(|path| Player::open(&path).map_err(|error| error.to_string()));
        match opened {
            Ok(player) => {
                log::info!("show: plays media {at} of slide {part}");
                player.play();
                self.drop_media();
                self.media = Some(Playing {
                    part,
                    index: at,
                    player,
                    frame: None,
                });
                self.notice = None;
                cx.notify();
            }
            Err(error) => self.media_failed(&error, cx),
        }
    }

    /// Takes the newest frame; true while frames may change.
    pub fn media_tick(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(playing) = self.media.as_mut() else {
            return false;
        };
        let repeat = self.slides[playing.part].media[playing.index].repeat;
        let ticked = playing.player.tick(repeat);
        let old = playing.player.frame().and_then(|frame| playing.frame.replace(picture(frame)));
        if let Some(old) = old {
            self.retire(old);
        }
        self.media_seen(ticked, cx)
    }

    /// Draws on, or stops and says why.
    pub fn media_seen(&mut self, ticked: Result<bool, String>, cx: &mut Context<Self>) -> bool {
        ticked.unwrap_or_else(|reason| {
            self.media_failed(&reason, cx);
            false
        })
    }

    /// Ends playback; its last picture is freed.
    fn drop_media(&mut self) {
        if let Some(picture) = self.media.take().and_then(|playing| playing.frame) {
            self.retire(picture);
        }
    }

    /// A picture drawn in each show window, freed in each.
    fn retire(&mut self, picture: Picture) {
        if self.presenter {
            retire(&mut self.retired[1], picture.clone());
        }
        retire(&mut self.retired[0], picture);
    }

    /// The playing video's frame and its bounds as shares.
    pub fn media_frame(&self) -> Option<(Picture, (f32, f32, f32, f32))> {
        let playing = self.media.as_ref()?;
        let (left, top, width, height) = self.slides[playing.part].media[playing.index].bounds;
        let (w, h) = self.slide;
        let share = (left as f32 / w, top as f32 / h, width as f32 / w, height as f32 / h);
        Some((playing.frame.clone()?, share))
    }
}
