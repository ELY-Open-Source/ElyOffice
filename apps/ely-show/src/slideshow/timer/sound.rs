//! The sounds a countdown ends with, made here.

use std::f32::consts::TAU;

use ely_platform::{AppId, media::Player};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sound {
    None,
    #[default]
    Bell,
    Alarm,
    Fanfare,
}

/// Each sound, its kept word, its label's key.
pub const SOUNDS: [(Sound, &str, &str); 4] = [
    (Sound::None, "none", "show.timer.sound.none"),
    (Sound::Bell, "bell", "show.timer.sound.bell"),
    (Sound::Alarm, "alarm", "show.timer.sound.alarm"),
    (Sound::Fanfare, "fanfare", "show.timer.sound.fanfare"),
];

/// Samples a second.
const RATE: u32 = 22_050;
/// The loudest sample, as a share of full scale.
const PEAK: f32 = 0.8;

/// A note: start and length in seconds, pitch, fading.
type Note = (f32, f32, f32, f32);

impl Sound {
    fn notes(self) -> Vec<Note> {
        match self {
            Sound::None => Vec::new(),
            // Three strikes that ring out.
            Sound::Bell => [0.0, 0.55, 1.1].iter().map(|&at| (at, 1.6, 1318.5, 3.0)).collect(),
            // Soft two-tone beeps, never harsh.
            Sound::Alarm => (0..10)
                .map(|n| (n as f32 * 0.24, 0.18, if n % 2 == 0 { 880.0 } else { 660.0 }, 1.0))
                .collect(),
            // A rising C chord, its top held.
            Sound::Fanfare => vec![
                (0.0, 0.18, 523.3, 2.0),
                (0.18, 0.18, 659.3, 2.0),
                (0.36, 0.18, 784.0, 2.0),
                (0.54, 1.4, 1046.5, 1.5),
                (0.54, 1.4, 784.0, 1.5),
                (0.54, 1.4, 659.3, 1.5),
            ],
        }
    }

    /// The sound as a mono 16-bit WAV file.
    fn wav(self) -> Vec<u8> {
        let notes = self.notes();
        let seconds = notes.iter().map(|(at, length, ..)| at + length).fold(0.0, f32::max);
        let mut mix = vec![0.0_f32; (seconds * RATE as f32) as usize];
        for (at, length, pitch, fading) in notes {
            let first = (at * RATE as f32) as usize;
            let count = (length * RATE as f32) as usize;
            for (n, sample) in mix.iter_mut().skip(first).take(count).enumerate() {
                let t = n as f32 / RATE as f32;
                // Short ramps at both ends avoid clicks.
                let ramp = (t / 0.005).min((length - t) / 0.02).clamp(0.0, 1.0);
                let wave = [(1.0, 1.0), (2.0, 0.35), (3.0, 0.15)]
                    .iter()
                    .map(|(times, loud)| (TAU * pitch * times * t).sin() * loud)
                    .sum::<f32>();
                *sample += wave * ramp * (-t * fading).exp();
            }
        }
        let loudest = mix.iter().fold(f32::EPSILON, |most, sample| most.max(sample.abs()));
        let bytes = mix.len() as u32 * 2;
        let mut file = Vec::with_capacity(44 + bytes as usize);
        file.extend_from_slice(b"RIFF");
        file.extend_from_slice(&(36 + bytes).to_le_bytes());
        file.extend_from_slice(b"WAVEfmt ");
        // PCM, one channel, two bytes a sample.
        file.extend_from_slice(&16_u32.to_le_bytes());
        file.extend_from_slice(&1_u16.to_le_bytes());
        file.extend_from_slice(&1_u16.to_le_bytes());
        file.extend_from_slice(&RATE.to_le_bytes());
        file.extend_from_slice(&(RATE * 2).to_le_bytes());
        file.extend_from_slice(&2_u16.to_le_bytes());
        file.extend_from_slice(&16_u16.to_le_bytes());
        file.extend_from_slice(b"data");
        file.extend_from_slice(&bytes.to_le_bytes());
        for sample in mix {
            let scaled = (sample / loudest * PEAK * f32::from(i16::MAX)) as i16;
            file.extend_from_slice(&scaled.to_le_bytes());
        }
        file
    }

    /// Starts the sound; `None` for silence.
    pub fn play(self) -> Result<Option<Player>, String> {
        let Some((_, word, _)) = SOUNDS.iter().find(|(sound, ..)| *sound == self && self != Sound::None) else {
            return Ok(None);
        };
        let said = |error: &dyn std::fmt::Display| format!("{error:#}");
        let folder = ely_platform::data_dir(AppId::Show).map_err(|error| said(&error))?.join("sounds");
        std::fs::create_dir_all(&folder).map_err(|error| said(&error))?;
        let path = folder.join(format!("{word}.wav"));
        std::fs::write(&path, self.wav()).map_err(|error| said(&error))?;
        let player = Player::open(&path).map_err(|error| said(&error))?;
        player.play();
        log::info!("show: countdown sound {word} plays");
        Ok(Some(player))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_sound_is_a_short_audible_wav() {
        for (sound, word, _) in SOUNDS {
            let file = sound.wav();
            assert_eq!((&file[..4], &file[8..16]), (&b"RIFF"[..], &b"WAVEfmt "[..]), "{word}");
            let samples: Vec<i16> = file[44..].as_chunks::<2>().0.iter().map(|pair| i16::from_le_bytes(*pair)).collect();
            let seconds = samples.len() as f32 / RATE as f32;
            let loudest = samples.iter().map(|sample| sample.unsigned_abs()).max().unwrap_or(0);
            match sound {
                Sound::None => assert!(samples.is_empty()),
                _ => assert!((1.5..4.0).contains(&seconds) && loudest > 20_000, "{word}: {seconds}s, {loudest}"),
            }
        }
    }
}
