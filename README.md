# ⏰ Loud Alarm

A loud, cross-platform desktop alarm built with **Tauri 2 + SolidJS + TypeScript**.
It is designed to actually wake you up:

- **Loud** - rings a piercing, procedurally generated siren on a loop until you stop it, at a volume level you choose.
- **Beats a muted / zero volume** - optionally snapshots your system volume, overrides it (unmuted) for the ring, and restores it exactly afterwards, so it wakes you without permanently changing your setting.
- **Never changes your volume (optional)** - turn the override off and the chosen level is applied as in-app gain only; your system volume is never touched.
- **Sounds through Focus / Do Not Disturb** - the alarm is played as *media audio*, not a notification. macOS Focus (and equivalent modes) silence notifications, not the media path, so the alarm still rings.
- **Keeps ringing after you close the window** - closing hides the app to a tray icon; the alarm still fires and brings the window back when it does.

> Status: **beta** (`v0.1.0-beta.1`). Works today on macOS; Linux uses PulseAudio/PipeWire for the volume override, Windows volume override is best-effort (see [Platform support](#platform-support)).

![CI](https://github.com/adipascu/loud-alarm/actions/workflows/ci.yml/badge.svg)

## Install

Grab an installer for your OS from the [Releases page](https://github.com/adipascu/loud-alarm/releases).

- **macOS** - `.dmg` (universal: Apple Silicon + Intel)
- **Linux** - `.AppImage` / `.deb`
- **Windows** - `.msi` / `.exe`

> The beta is unsigned, so on first launch macOS may require right-click → **Open**, and Windows SmartScreen may need **More info → Run anyway**.

## Usage

1. Launch **Loud Alarm**.
2. Pick a time (defaults to `07:00`), choose a sound (Siren / Beep / Chirp), set the **Volume** slider, and **Preview** it.
3. Leave **Override system volume** on so it rings even if the machine is muted (your volume is restored when it stops). Turn it off to keep your system volume untouched and use in-app gain only.
4. Click **Arm alarm**. The window shows a live countdown.
5. You can close the window: the app keeps running in the tray and still rings. When it fires, the window reappears as a flashing **RINGING** screen - hit **Stop** (it also auto-stops after 5 minutes). Quit fully from the tray menu.

> The app process must be running for the alarm to fire (closing the window is fine; quitting is not). It does **not** wake a sleeping machine; keep the Mac awake until the alarm time.

## How it meets the "wake me up" requirements

| Requirement | How |
|---|---|
| Loud, configurable | Square-wave siren looped continuously; volume set by the slider |
| Beats muted / 0 volume | When override is on, `volume::snapshot()` saves your volume + mute, `volume::apply()` forces the chosen level (unmuted) each burst, and the saved state is restored when ringing ends |
| Without changing system volume | With override off, the level is applied as in-app gain (`Sink::set_volume`); the system volume is never read or written |
| Sounds through Focus / DND | Audio plays via the media path (rodio → CoreAudio/WASAPI/ALSA); Focus only gates notifications |
| Rings after window close | A background scheduler thread lives for the whole process; closing only hides the window to the tray |

## Develop

Prerequisites: [Rust](https://rustup.rs), [Node 20+](https://nodejs.org), [pnpm](https://pnpm.io), and the [Tauri 2 system dependencies](https://tauri.app/start/prerequisites/).

```sh
pnpm install
pnpm tauri dev      # run the app with hot reload
```

## Test, lint, typecheck

```sh
pnpm test           # frontend unit tests (Vitest)
pnpm lint           # Biome lint + format check
pnpm typecheck      # tsc --noEmit
cargo test --manifest-path src-tauri/Cargo.toml   # Rust unit tests
```

## Build

```sh
pnpm tauri build    # produces native installers in src-tauri/target/release/bundle
```

## Platform support

| Platform | Audio | Volume override |
|---|---|---|
| macOS | ✅ | ✅ `osascript` |
| Linux | ✅ | ✅ PulseAudio/PipeWire (`pactl`) |
| Windows | ✅ | ⚠️ best-effort (volume-up key events); native mixer control is a follow-up |

## Architecture

- `src-tauri/src/siren.rs` - procedurally synthesised tones (no shipped audio assets).
- `src-tauri/src/volume.rs` - cross-platform system-volume snapshot / override / restore.
- `src-tauri/src/schedule.rs` - pure "next occurrence of HH:MM" logic (unit-tested).
- `src-tauri/src/alarm.rs` - the engine: scheduler thread + ring thread + state.
- `src/` - SolidJS + TypeScript UI that talks to the Rust commands.

## License

[MIT](./LICENSE) © 2026 Loud Alarm contributors
