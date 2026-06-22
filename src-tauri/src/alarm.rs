//! The alarm engine: holds the armed state, ticks a scheduler thread, and on
//! fire spins up an audio thread that rings until stopped.
//!
//! Threading model:
//! - one long-lived scheduler thread polls the clock once per second;
//! - when the target time passes it flips `ringing` and spawns a short-lived
//!   ring thread which owns its own audio stream (cpal streams are `!Send`, so
//!   the stream must be created and dropped on that thread);
//! - `stop`/`disarm` just flip flags; the ring thread observes them and exits.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use rodio::{OutputStream, Sink};

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

#[derive(Default)]
struct State {
    armed: bool,
    target: Option<DateTime<Local>>,
    hour: u32,
    minute: u32,
    sound: SoundKind,
    force_volume: bool,
    ringing: bool,
    stop: Option<Arc<AtomicBool>>,
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

    /// Play the chosen sound briefly at the current volume, for the UI preview
    /// button. Never touches the system volume.
    pub fn preview(&self, sound: SoundKind) {
        thread::spawn(move || {
            if let Ok((_stream, handle)) = OutputStream::try_default() {
                if let Ok(sink) = Sink::try_new(&handle) {
                    sink.append(Tone::new(sound, Duration::from_millis(1200)));
                    sink.sleep_until_end();
                }
            }
        });
    }

    /// Start the single background scheduler thread. Call once at startup.
    pub fn start_scheduler(self: &Arc<Self>) {
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
                    Some((state.sound, state.force_volume, flag))
                } else {
                    None
                }
            };

            if let Some((sound, force_volume, flag)) = fire {
                let engine = Arc::clone(&engine);
                thread::spawn(move || engine.ring(sound, force_volume, flag));
            }
        });
    }

    fn ring(&self, sound: SoundKind, force_volume: bool, stop: Arc<AtomicBool>) {
        let started = Instant::now();
        if let Ok((_stream, handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&handle) {
                while !stop.load(Ordering::SeqCst) && started.elapsed().as_secs() < MAX_RING_SECS {
                    if force_volume {
                        let _ = volume::maximize();
                    }
                    sink.append(Tone::new(sound, RING_BURST));
                    sink.sleep_until_end();
                }
                sink.stop();
            }
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
        let status = engine.arm(17, 0, SoundKind::Siren, true).unwrap();
        assert!(status.armed);
        assert_eq!(status.hour, 17);
        assert!(status.force_volume);
        assert!(status.seconds_remaining > 0);

        let status = engine.disarm();
        assert!(!status.armed);
    }

    #[test]
    fn arm_rejects_invalid_time() {
        let engine = AlarmEngine::new();
        assert!(engine.arm(25, 0, SoundKind::Beep, true).is_err());
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
