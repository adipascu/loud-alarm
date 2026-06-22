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
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, State, WindowEvent};

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

/// Bring the main window back to the foreground.
fn show_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let engine = AlarmEngine::new();

    tauri::Builder::default()
        .manage(engine)
        .setup(|app| {
            // Keep ticking in the background for the life of the process.
            let handle = app.handle().clone();
            app.state::<Engine>()
                .inner()
                .clone()
                .start_scheduler(handle);

            // Tray icon so the app can run with no visible window and still be
            // reachable (Show) and quittable (Quit).
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("main-tray")
                .tooltip("Loud Alarm")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it instead of quitting, so the alarm
            // keeps running in the background.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
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
