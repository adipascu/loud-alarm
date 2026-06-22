//! Cross-platform system-volume control with snapshot/restore.
//!
//! There is no public macOS API (Accessibility or otherwise) that lets a normal
//! app play *through* a hardware mute or *above* the system volume without
//! changing it. So to ring loudly even when the Mac is muted while still
//! respecting the user's setting, we snapshot the current volume + mute state,
//! override it for the duration of the alarm, and restore it exactly afterwards.
//!
//! `set_commands` is split out so command construction can be unit-tested
//! without changing the tester's volume.

use std::process::Command;

/// A single external command to run, as (program, args).
pub type VolumeCommand = (String, Vec<String>);

/// A captured system output volume state, used to restore after ringing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemVolume {
    pub level: u8,
    pub muted: bool,
}

/// Commands that set the system output volume to `level` (0-100) and mute state.
pub fn set_commands(level: u8, muted: bool) -> Vec<VolumeCommand> {
    let level = level.min(100);

    #[cfg(target_os = "macos")]
    {
        let muted_script = format!("set volume output muted {muted}");
        let volume_script = format!("set volume output volume {level}");
        vec![(
            "osascript".to_string(),
            vec![
                "-e".to_string(),
                muted_script,
                "-e".to_string(),
                volume_script,
            ],
        )]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            (
                "pactl".to_string(),
                vec![
                    "set-sink-mute".to_string(),
                    "@DEFAULT_SINK@".to_string(),
                    (if muted { "1" } else { "0" }).to_string(),
                ],
            ),
            (
                "pactl".to_string(),
                vec![
                    "set-sink-volume".to_string(),
                    "@DEFAULT_SINK@".to_string(),
                    format!("{level}%"),
                ],
            ),
        ]
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        // Reliable absolute volume control on Windows needs a native mixer
        // crate; tracked as a follow-up. Loudness still comes from in-app gain.
        let _ = (level, muted);
        Vec::new()
    }
}

/// Read the current system output volume and mute state. Returns `None` on
/// platforms where it cannot be queried (so the caller skips restoring).
pub fn snapshot() -> Option<SystemVolume> {
    #[cfg(target_os = "macos")]
    {
        let level = osascript_value("output volume of (get volume settings)")?
            .trim()
            .parse::<u8>()
            .ok()?;
        let muted = osascript_value("output muted of (get volume settings)")?
            .trim()
            .eq_ignore_ascii_case("true");
        Some(SystemVolume { level, muted })
    }

    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Set the system output volume and mute state. Errors are non-fatal: a failure
/// here should never stop the alarm from trying to ring.
pub fn apply(level: u8, muted: bool) -> Result<(), String> {
    let mut errors = Vec::new();
    for (program, args) in set_commands(level, muted) {
        match Command::new(&program).args(&args).output() {
            Ok(out) if !out.status.success() => errors.push(format!(
                "{program} exited with {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            )),
            Err(e) => errors.push(format!("failed to run {program}: {e}")),
            Ok(_) => {}
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

#[cfg(target_os = "macos")]
fn osascript_value(script: &str) -> Option<String> {
    let out = Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_level_to_100() {
        let commands = set_commands(250, false);
        let joined = commands
            .iter()
            .flat_map(|(_, args)| args.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            assert!(joined.contains("100"));
            assert!(!joined.contains("250"));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_sets_volume_and_unmutes() {
        let commands = set_commands(80, false);
        assert_eq!(commands.len(), 1);
        let (program, args) = &commands[0];
        assert_eq!(program, "osascript");
        let joined = args.join(" ");
        assert!(joined.contains("set volume output volume 80"));
        assert!(joined.contains("set volume output muted false"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_sets_volume_and_unmutes() {
        let commands = set_commands(80, false);
        let joined: String = commands
            .iter()
            .map(|(p, a)| format!("{p} {}", a.join(" ")))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(joined.contains("set-sink-volume"));
        assert!(joined.contains("80%"));
        assert!(joined.contains("set-sink-mute @DEFAULT_SINK@ 0"));
    }
}
