//! Loud Alarm: a cross-platform alarm that beats a muted system volume and
//! sounds through Focus / Do Not Disturb.

mod alarm;
mod schedule;
mod siren;
mod volume;

use std::str::FromStr;
use std::sync::Arc;

use alarm::{AlarmEngine, Status};
use siren::SoundKind;
use tauri::{Manager, State};

type Engine = Arc<AlarmEngine>;

#[tauri::command]
fn arm_alarm(
    engine: State<'_, Engine>,
    hour: u32,
    minute: u32,
    sound: String,
    force_volume: bool,
) -> Result<Status, String> {
    let kind = SoundKind::from_str(&sound)?;
    engine.arm(hour, minute, kind, force_volume)
}

#[tauri::command]
fn disarm_alarm(engine: State<'_, Engine>) -> Status {
    engine.disarm()
}

#[tauri::command]
fn stop_ringing(engine: State<'_, Engine>) -> Status {
    engine.stop()
}

#[tauri::command]
fn get_status(engine: State<'_, Engine>) -> Status {
    engine.status()
}

#[tauri::command]
fn preview_sound(engine: State<'_, Engine>, sound: String) -> Result<(), String> {
    let kind = SoundKind::from_str(&sound)?;
    engine.preview(kind);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine = AlarmEngine::new();

    tauri::Builder::default()
        .manage(engine)
        .setup(|app| {
            let engine = app.state::<Engine>().inner().clone();
            engine.start_scheduler();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            arm_alarm,
            disarm_alarm,
            stop_ringing,
            get_status,
            preview_sound
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
