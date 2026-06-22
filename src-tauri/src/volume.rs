//! Cross-platform "make the system audible no matter what" control.
//!
//! The alarm must ring even when the machine is muted or the output volume is
//! at zero, so before every burst we force the OS output volume to maximum and
//! unmute it. The platform-specific shell incantations live here.
//!
//! `volume_commands` is split out from `maximize` so the command construction
//! can be unit-tested without actually changing the tester's volume.

use std::process::Command;

/// A single external command to run, as (program, args).
pub type VolumeCommand = (String, Vec<String>);

fn cmd(program: &str, args: &[&str]) -> VolumeCommand {
    (
        program.to_string(),
        args.iter().map(|a| a.to_string()).collect(),
    )
}

/// The ordered list of commands that unmute and maximize the system output
/// volume on the current platform.
pub fn volume_commands() -> Vec<VolumeCommand> {
    #[cfg(target_os = "macos")]
    {
        vec![cmd(
            "osascript",
            &[
                "-e",
                "set volume output muted false",
                "-e",
                "set volume output volume 100",
            ],
        )]
    }

    #[cfg(target_os = "linux")]
    {
        // PulseAudio / PipeWire via pactl; covers the vast majority of desktops.
        vec![
            cmd("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "0"]),
            cmd("pactl", &["set-sink-volume", "@DEFAULT_SINK@", "100%"]),
        ]
    }

    #[cfg(target_os = "windows")]
    {
        // Best effort: nudge the master volume up via PowerShell key events.
        // Reliable absolute control on Windows needs a native mixer crate;
        // tracked as a follow-up. Audio still plays at the current level.
        let script = "$o = New-Object -ComObject WScript.Shell; \
                      1..50 | ForEach-Object { $o.SendKeys([char]175) }";
        vec![cmd("powershell", &["-NoProfile", "-Command", script])]
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Vec::new()
    }
}

/// Force the system output volume to maximum and unmute it. Errors from
/// individual commands are collected but non-fatal: a failure to raise the
/// volume should never stop the alarm from trying to ring.
pub fn maximize() -> Result<(), String> {
    let mut errors = Vec::new();
    for (program, args) in volume_commands() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_at_least_one_command_on_supported_platforms() {
        let commands = volume_commands();
        if cfg!(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )) {
            assert!(!commands.is_empty());
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_unmutes_and_sets_full_volume() {
        let commands = volume_commands();
        assert_eq!(commands.len(), 1);
        let (program, args) = &commands[0];
        assert_eq!(program, "osascript");
        let joined = args.join(" ");
        assert!(joined.contains("set volume output muted false"));
        assert!(joined.contains("set volume output volume 100"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_unmutes_and_sets_full_volume() {
        let commands = volume_commands();
        let joined: String = commands
            .iter()
            .map(|(p, a)| format!("{p} {}", a.join(" ")))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(joined.contains("set-sink-mute"));
        assert!(joined.contains("100%"));
    }
}
