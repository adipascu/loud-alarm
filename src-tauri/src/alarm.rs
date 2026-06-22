//! The alarm engine: holds the armed state, ticks a scheduler thread, and on
//! fire spins up an audio thread that rings until stopped.
//!
//! Threading model:
//! - one long-lived scheduler thread polls the clock once per second;
//! - when the target time passes it flips `ringing` and spawns a short-lived
//!   ring thread which owns its own audio stream (cpal streams are `!Send`, so
//!   the stream must be created and dropped on that thread);
//! - `stop`/`disarm` just flip flags; the ring thread observes them and exits.
//!
//! The scheduler keeps running as long as the process is alive, independent of
//! whether any window is visible, and the ring thread surfaces the main window
//! so the alarm is visible (and stoppable) even if it was hidden to the tray.
//!
//! Loudness has two modes:
//! - `force_volume`: snapshot the system volume, override it to `volume_level`
//!   (and unmute) for the ring, then restore it exactly afterwards;
//! - otherwise: leave the system volume untouched and apply `volume_level` as
//!   in-app playback gain only.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use rodio::{OutputStream, Sink};
use tauri::{AppHandle, Manager};

use crate::schedule;
use crate::siren::{SoundKind, Tone};
use crate::volume;

/// Length of a single ring burst before volume is re-asserted.
const RING_BURST: Duration = Duration::from_millis(1500);
/// Safety net so a forgotten alarm doesn't ring forever.
const MAX_RING_SECS: u64 = 300;

#[derive(Default)]
pub struct AlarmEngine {
    inner: Mutex<State>,
}

struct State {
    armed: bool,
    target: Option<DateTime<Local>>,
    hour: u32,
    minute: u32,
    sound: SoundKind,
    force_volume: bool,
    volume_level: u8,
    ringing: bool,
    stop: Option<Arc<AtomicBool>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            armed: false,
            target: None,
            hour: 0,
            minute: 0,
            sound: SoundKind::default(),
            force_volume: true,
            volume_level: 100,
            ringing: false,
            stop: None,
        }
    }
}

/// What the ring thread needs, captured atomically at fire time.
struct RingConfig {
    sound: SoundKind,
    force_volume: bool,
    volume_level: u8,
    stop: Arc<AtomicBool>,
}

/// Snapshot of the engine for the UI.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub armed: bool,
    pub ringing: bool,
    pub hour: u32,
    pub minute: u32,
    pub seconds_remaining: i64,
    pub force_volume: bool,
    pub volume_level: u8,
    pub sound: SoundKind,
}

impl AlarmEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn snapshot(state: &State) -> Status {
        let remaining = match (state.armed, state.target) {
            (true, Some(target)) => schedule::seconds_until(Local::now(), target),
            _ => 0,
        };
        Status {
            armed: state.armed,
            ringing: state.ringing,
            hour: state.hour,
            minute: state.minute,
            seconds_remaining: remaining,
            force_volume: state.force_volume,
            volume_level: state.volume_level,
            sound: state.sound,
        }
    }

    pub fn status(&self) -> Status {
        Self::snapshot(&self.inner.lock().unwrap())
    }

    pub fn arm(
        &self,
        hour: u32,
        minute: u32,
        sound: SoundKind,
        force_volume: bool,
        volume_level: u8,
    ) -> Result<Status, String> {
        schedule::validate(hour, minute)?;
        let target = schedule::next_occurrence(Local::now(), hour, minute);

        let mut state = self.inner.lock().unwrap();
        state.armed = true;
        state.target = Some(target);
        state.hour = hour;
        state.minute = minute;
        state.sound = sound;
        state.force_volume = force_volume;
        state.volume_level = volume_level.min(100);
        Ok(Self::snapshot(&state))
    }

    pub fn disarm(&self) -> Status {
        let mut state = self.inner.lock().unwrap();
        state.armed = false;
        Self::snapshot(&state)
    }

    pub fn stop(&self) -> Status {
        let mut state = self.inner.lock().unwrap();
        if let Some(flag) = state.stop.take() {
            flag.store(true, Ordering::SeqCst);
        }
        state.ringing = false;
        Self::snapshot(&state)
    }

    /// Play the chosen sound briefly at the given in-app gain, for the UI
    /// preview button. Never touches the system volume.
    pub fn preview(&self, sound: SoundKind, volume_level: u8) {
        thread::spawn(move || {
            if let Ok((_stream, handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&handle) {
                    sink.set_volume(f32::from(volume_level.min(100)) / 100.0);
                    sink.append(Tone::new(sound, Duration::from_millis(1200)));
                    sink.sleep_until_end();
                }
            }
        });
    }

    /// Start the single background scheduler thread. Call once at startup.
    pub fn start_scheduler(self: &Arc<Self>, app: AppHandle) {
        let engine = Arc::clone(self);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(1));

            let fire = {
                let mut state = engine.inner.lock().unwrap();
                let due = state.armed
                    && !state.ringing
                    && state.target.is_some_and(|t| Local::now() >= t);
                if due {
                    state.armed = false;
                    state.ringing = true;
                    let flag = Arc::new(AtomicBool::new(false));
                    state.stop = Some(Arc::clone(&flag));
                    Some(RingConfig {
                        sound: state.sound,
                        force_volume: state.force_volume,
                        volume_level: state.volume_level,
                        stop: flag,
                    })
                } else {
                    None
                }
            };

            if let Some(config) = fire {
                let engine = Arc::clone(&engine);
                let app = app.clone();
                thread::spawn(move || engine.ring(config, app));
            }
        });
    }

    fn ring(&self, config: RingConfig, app: AppHandle) {
        // Surface the window so the user can see the alarm and hit Stop, even
        // if it was hidden to the tray.
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }

        // Borrow the system volume only if asked; restore it exactly afterwards.
        let saved = if config.force_volume {
            volume::snapshot()
        } else {
            None
        };

        let started = Instant::now();
        if let Ok((_stream, handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&handle) {
                if !config.force_volume {
                    sink.set_volume(f32::from(config.volume_level) / 100.0);
                }
                while !config.stop.load(Ordering::SeqCst)
                    && started.elapsed().as_secs() < MAX_RING_SECS
                {
                    if config.force_volume {
                        let _ = volume::apply(config.volume_level, false);
                    }
                    sink.append(Tone::new(config.sound, RING_BURST));
                    sink.sleep_until_end();
                }
                sink.stop();
            }
        }

        if let Some(saved) = saved {
            let _ = volume::apply(saved.level, saved.muted);
        }

        // Ringing finished (stopped, timed out, or no audio device).
        let mut state = self.inner.lock().unwrap();
        state.ringing = false;
        state.stop = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_then_disarm_updates_status() {
        let engine = AlarmEngine::new();
        let status = engine.arm(17, 0, SoundKind::Siren, true, 80).unwrap();
        assert!(status.armed);
        assert_eq!(status.hour, 17);
        assert!(status.force_volume);
        assert_eq!(status.volume_level, 80);
        assert!(status.seconds_remaining > 0);

        let status = engine.disarm();
        assert!(!status.armed);
    }

    #[test]
    fn arm_rejects_invalid_time() {
        let engine = AlarmEngine::new();
        assert!(engine.arm(25, 0, SoundKind::Beep, true, 100).is_err());
    }

    #[test]
    fn arm_clamps_volume_to_100() {
        let engine = AlarmEngine::new();
        let status = engine.arm(8, 30, SoundKind::Chirp, true, 250).unwrap();
        assert_eq!(status.volume_level, 100);
    }

    #[test]
    fn stop_clears_ringing() {
        let engine = AlarmEngine::new();
        {
            let mut state = engine.inner.lock().unwrap();
            state.ringing = true;
            state.stop = Some(Arc::new(AtomicBool::new(false)));
        }
        let status = engine.stop();
        assert!(!status.ringing);
    }
}
