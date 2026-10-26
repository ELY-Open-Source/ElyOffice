//! Plays local audio and video with the system's player (SHOW-10).

use std::path::Path;

#[cfg(target_os = "macos")]
mod macos;

/// Only macOS plays yet; elsewhere nothing offers it.
pub const BUILT: bool = cfg!(target_os = "macos");

/// What the player knows of its file.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Loading,
    /// Seconds long, and the picture size of a video.
    Ready {
        seconds: f64,
        video: Option<(u32, u32)>,
    },
    /// The system cannot play it; the reason is its own.
    Failed(String),
}

/// A picture of the playing video.
#[cfg(target_os = "macos")]
pub type Frame = objc2_core_foundation::CFRetained<objc2_core_video::CVPixelBuffer>;
/// No platform here makes frames.
#[cfg(not(target_os = "macos"))]
pub enum Frame {}

/// One file in the system player, paused at first.
pub struct Player {
    /// Paused by a person, not at its end.
    held: std::cell::Cell<bool>,
    #[cfg(target_os = "macos")]
    inner: macos::Player,
    // Nothing opens a player where none is built.
    #[cfg(not(target_os = "macos"))]
    inner: std::convert::Infallible,
}

impl Player {
    /// Opens `path`; a file that is not there fails here.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if !path.is_file() {
            anyhow::bail!("{} is not there", path.display());
        }
        log::info!("media: opens a file");
        #[cfg(target_os = "macos")]
        return Ok(Player {
            held: std::cell::Cell::new(false),
            inner: macos::Player::open(path)?,
        });
        #[cfg(not(target_os = "macos"))]
        anyhow::bail!("playing {} is not built on this platform", path.display())
    }

    pub fn state(&self) -> State {
        self.inner.state()
    }

    pub fn play(&self) {
        self.held.set(false);
        self.inner.play()
    }

    pub fn pause(&self) {
        self.held.set(true);
        self.inner.pause()
    }

    pub fn playing(&self) -> bool {
        self.inner.playing()
    }

    /// Where it is, in seconds.
    pub fn position(&self) -> f64 {
        self.inner.position()
    }

    pub fn seek(&self, seconds: f64) {
        self.inner.seek(seconds)
    }

    /// The picture due now, when a new one is.
    pub fn frame(&self) -> Option<Frame> {
        self.inner.frame()
    }

    /// It stopped at its end, not by a pause.
    pub fn ended(&self) -> bool {
        matches!(self.state(), State::Ready { seconds, .. }
            if !self.held.get() && !self.playing() && self.position() >= seconds - 0.05)
    }

    /// Restarts a loop; true while frames change.
    pub fn tick(&self, repeat: bool) -> Result<bool, String> {
        if repeat && self.ended() {
            self.seek(0.0);
            self.play();
        }
        match self.state() {
            State::Loading => Ok(true),
            State::Ready { .. } => Ok(self.playing()),
            State::Failed(reason) => Err(reason),
        }
    }
}

/// What an uninhabited player answers: nothing, ever.
#[cfg(not(target_os = "macos"))]
trait Never {
    fn state(&self) -> State;
    fn play(&self);
    fn pause(&self);
    fn playing(&self) -> bool;
    fn position(&self) -> f64;
    fn seek(&self, seconds: f64);
    fn frame(&self) -> Option<Frame>;
}

#[cfg(not(target_os = "macos"))]
impl Never for std::convert::Infallible {
    fn state(&self) -> State {
        match *self {}
    }
    fn play(&self) {
        match *self {}
    }
    fn pause(&self) {
        match *self {}
    }
    fn playing(&self) -> bool {
        match *self {}
    }
    fn position(&self) -> f64 {
        match *self {}
    }
    fn seek(&self, _: f64) {
        match *self {}
    }
    fn frame(&self) -> Option<Frame> {
        match *self {}
    }
}
