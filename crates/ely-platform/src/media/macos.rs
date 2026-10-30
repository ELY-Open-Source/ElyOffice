use std::{path::Path, ptr::NonNull};

use objc2::{AllocAnyThread, MainThreadMarker, rc::Retained, runtime::AnyObject};
use objc2_av_foundation::{
    AVMediaTypeAudio, AVMediaTypeVideo, AVPlayer, AVPlayerItem, AVPlayerItemOutput,
    AVPlayerItemStatus, AVPlayerItemVideoOutput,
};
use objc2_core_foundation::CFRetained;
use objc2_core_media::{CMTime, CMTimeFlags};
use objc2_core_video::kCVPixelBufferPixelFormatTypeKey;
use objc2_foundation::{NSDictionary, NSNumber, NSString, NSURL};

use super::State;

/// Biplanar full-range YCbCr, as GPUI's surfaces draw.
const PIXELS: u32 = u32::from_be_bytes(*b"420f");

pub struct Player {
    player: Retained<AVPlayer>,
    item: Retained<AVPlayerItem>,
    output: Retained<AVPlayerItemVideoOutput>,
}

fn seconds(time: CMTime) -> Option<f64> {
    (time.flags.contains(CMTimeFlags::Valid) && !time.flags.contains(CMTimeFlags::Indefinite))
        .then(|| time.value as f64 / time.timescale as f64)
}

impl Player {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| anyhow::anyhow!("media plays on the main thread"))?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
        let player = unsafe { AVPlayer::playerWithURL(&url, mtm) };
        let item = unsafe { player.currentItem() }
            .ok_or_else(|| anyhow::anyhow!("no player item for {}", path.display()))?;
        // The key is a CFString; NSString is its toll-free twin.
        let key: &NSString =
            unsafe { &*(kCVPixelBufferPixelFormatTypeKey as *const _ as *const NSString) };
        let format = NSNumber::new_u32(PIXELS);
        let value: &AnyObject = &format;
        let attributes = NSDictionary::<NSString, AnyObject>::from_slices(&[key], &[value]);
        let output = unsafe {
            AVPlayerItemVideoOutput::initWithPixelBufferAttributes(
                AVPlayerItemVideoOutput::alloc(),
                Some(&attributes),
            )
        };
        unsafe { item.addOutput(&output as &AVPlayerItemOutput) };
        Ok(Player {
            player,
            item,
            output,
        })
    }

    pub fn state(&self) -> State {
        match unsafe { self.item.status() } {
            AVPlayerItemStatus::ReadyToPlay => {
                if let Some(track) = self.unplayable() {
                    return State::Failed(format!("its {track} track is in a format this system cannot play"));
                }
                let size = unsafe { self.item.presentationSize() };
                let video = (size.width > 0.0 && size.height > 0.0)
                    .then(|| (size.width.round() as u32, size.height.round() as u32));
                State::Ready {
                    seconds: seconds(unsafe { self.item.duration() }).unwrap_or(0.0),
                    video,
                }
            }
            AVPlayerItemStatus::Failed => {
                let reason = unsafe { self.item.error() }
                    .map(|error| error.localizedDescription().to_string())
                    .unwrap_or_else(|| "the system player gave no reason".into());
                State::Failed(reason)
            }
            _ => State::Loading,
        }
    }

    /// A track the system cannot play: no half playback.
    fn unplayable(&self) -> Option<String> {
        let tracks = unsafe { self.item.asset().tracks() };
        let track = tracks.iter().find(|track| !unsafe { track.isPlayable() })?;
        let kind = unsafe { track.mediaType() };
        let is = |known: Option<&NSString>| known.is_some_and(|known| *known == *kind);
        Some(if is(unsafe { AVMediaTypeVideo }) {
            "video".into()
        } else if is(unsafe { AVMediaTypeAudio }) {
            "sound".into()
        } else {
            kind.to_string()
        })
    }

    pub fn play(&self) {
        unsafe { self.player.play() }
    }

    pub fn pause(&self) {
        unsafe { self.player.pause() }
    }

    pub fn playing(&self) -> bool {
        (unsafe { self.player.rate() }) > 0.0
    }

    pub fn position(&self) -> f64 {
        seconds(unsafe { self.player.currentTime() }).unwrap_or(0.0)
    }

    pub fn seek(&self, to: f64) {
        let time = CMTime {
            value: (to * 600.0).round() as i64,
            timescale: 600,
            flags: CMTimeFlags::Valid,
            epoch: 0,
        };
        let exact = CMTime {
            value: 0,
            timescale: 1,
            flags: CMTimeFlags::Valid,
            epoch: 0,
        };
        unsafe {
            self.player
                .seekToTime_toleranceBefore_toleranceAfter(time, exact, exact)
        }
    }

    pub fn frame(&self) -> Option<super::Frame> {
        let now = unsafe { self.item.currentTime() };
        if !unsafe { self.output.hasNewPixelBufferForItemTime(now) } {
            return None;
        }
        let buffer = unsafe {
            self.output
                .copyPixelBufferForItemTime_itemTimeForDisplay(now, std::ptr::null_mut())
        }?;
        let raw = Retained::into_raw(buffer);
        // The copy hands its one reference over.
        Some(unsafe { CFRetained::from_raw(NonNull::new_unchecked(raw)) })
    }
}
