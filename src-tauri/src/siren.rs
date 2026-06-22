//! Procedurally generated alarm tones.
//!
//! The sound is synthesised at runtime rather than shipped as an audio file:
//! it keeps the repository free of third-party media (and its licensing), and
//! it is trivially cross-platform. Crucially, the tone is played through the
//! normal audio output (rodio/cpal -> CoreAudio/WASAPI/ALSA) as *media*, not as
//! a notification, which is exactly why Focus / Do Not Disturb does not silence
//! it: those modes gate notifications, not the media path.

use std::str::FromStr;
use std::time::Duration;

const SAMPLE_RATE: u32 = 44_100;
/// Kept just below full scale so the square edges don't hard-clip the DAC into
/// mush; final loudness is set by the system volume or the in-app gain.
const AMPLITUDE: f32 = 0.8;

/// The selectable alarm sounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SoundKind {
    /// Classic two-tone "wee-woo" emergency siren.
    #[default]
    Siren,
    /// Fast, piercing on/off beeps.
    Beep,
    /// A pitch that ramps upward and resets, urgent and hard to sleep through.
    Chirp,
}

impl FromStr for SoundKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "siren" => Ok(SoundKind::Siren),
            "beep" => Ok(SoundKind::Beep),
            "chirp" => Ok(SoundKind::Chirp),
            other => Err(format!("unknown sound '{other}'")),
        }
    }
}

/// A finite, mono, 32-bit-float tone source for rodio.
///
/// Phase is integrated sample-by-sample so frequency sweeps stay continuous
/// (no clicks from naive `sin(2*pi*f(t)*t)`).
pub struct Tone {
    kind: SoundKind,
    total_samples: usize,
    pos: usize,
    phase: f64,
}

impl Tone {
    pub fn new(kind: SoundKind, duration: Duration) -> Self {
        let total_samples = (duration.as_secs_f64() * SAMPLE_RATE as f64) as usize;
        Self {
            kind,
            total_samples,
            pos: 0,
            phase: 0.0,
        }
    }

    /// Instantaneous frequency (Hz) for the current time `t` seconds.
    fn instantaneous_freq(&self, t: f64) -> f64 {
        match self.kind {
            // 600 Hz <-> 900 Hz oscillating ~twice per second.
            SoundKind::Siren => 750.0 + 150.0 * (2.0 * std::f64::consts::PI * 2.0 * t).sin(),
            // Steady 1000 Hz tone; gating handled in `next`.
            SoundKind::Beep => 1000.0,
            // 500 Hz -> 1500 Hz ramp that resets every 0.5 s.
            SoundKind::Chirp => 500.0 + 1000.0 * ((t % 0.5) / 0.5),
        }
    }

    /// On/off envelope (1.0 = audible, 0.0 = silent) for the current time.
    fn gate(&self, t: f64) -> f32 {
        match self.kind {
            SoundKind::Beep => {
                // 8 Hz on/off: audible for the first half of each 125 ms cycle.
                if (t * 8.0).fract() < 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
            _ => 1.0,
        }
    }
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.total_samples {
            return None;
        }
        let t = self.pos as f64 / SAMPLE_RATE as f64;
        let freq = self.instantaneous_freq(t);
        self.phase += 2.0 * std::f64::consts::PI * freq / SAMPLE_RATE as f64;
        // Square wave: harsher and more wake-inducing than a pure sine.
        let wave = if self.phase.sin() >= 0.0 { 1.0 } else { -1.0 };
        self.pos += 1;
        Some(wave * AMPLITUDE * self.gate(t))
    }
}

impl rodio::Source for Tone {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.total_samples - self.pos)
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.total_samples as f64 / SAMPLE_RATE as f64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_sounds_case_insensitively() {
        assert_eq!(SoundKind::from_str("Siren").unwrap(), SoundKind::Siren);
        assert_eq!(SoundKind::from_str("BEEP").unwrap(), SoundKind::Beep);
        assert_eq!(SoundKind::from_str("chirp").unwrap(), SoundKind::Chirp);
        assert!(SoundKind::from_str("kazoo").is_err());
    }

    #[test]
    fn tone_yields_expected_sample_count() {
        let tone = Tone::new(SoundKind::Siren, Duration::from_millis(100));
        let expected = SAMPLE_RATE as usize / 10;
        assert_eq!(tone.count(), expected);
    }

    #[test]
    fn tone_stays_within_amplitude_bounds() {
        let tone = Tone::new(SoundKind::Chirp, Duration::from_millis(50));
        for sample in tone {
            assert!(sample.abs() <= AMPLITUDE + f32::EPSILON);
        }
    }

    #[test]
    fn beep_actually_goes_silent_between_pulses() {
        let samples: Vec<f32> = Tone::new(SoundKind::Beep, Duration::from_millis(250)).collect();
        assert!(samples.contains(&0.0), "beep should have gaps");
        assert!(samples.iter().any(|s| *s != 0.0), "beep should have sound");
    }
}
