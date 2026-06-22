# ⏰ Loud Alarm

A loud, cross-platform desktop alarm built with **Tauri 2 + SolidJS + TypeScript**.
It is designed to actually wake you up:

- **Loud** - rings a piercing, procedurally generated siren on a loop until you stop it.
- **Beats a muted / zero volume** - before every ring it force-unmutes the system and drives the output volume to maximum.
- **Sounds through Focus / Do Not Disturb** - the alarm is played as *media audio*, not a notification. macOS Focus (and equivalent modes) silence notifications, not the media path, so the alarm still rings.

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
2. Pick a time (defaults to `17:00`), choose a sound (Siren / Beep / Chirp), and **Preview** it.
3. Leave **Force max volume** on so it rings even if the machine is muted.
4. Click **Arm alarm**. The window shows a live countdown.
5. When it fires, the whole window turns into a flashing **RINGING** screen - hit **Stop** (it also auto-stops after 5 minutes).

> The app must be running for the alarm to fire. It does **not** try to wake a sleeping machine; keep it open (e.g. minimized) until the alarm time.

## How it meets the "wake me up" requirements

| Requirement | How |
|---|---|
| Loud | Square-wave siren looped continuously through the audio device |
| Beats muted / 0 volume | `volume::maximize()` force-unmutes and sets output to 100% before every burst, re-asserting each loop |
| Sounds through Focus / DND | Audio plays via the media path (rodio → CoreAudio/WASAPI/ALSA); Focus only gates notifications |

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
- `src-tauri/src/volume.rs` - cross-platform "make it audible" volume control.
- `src-tauri/src/schedule.rs` - pure "next occurrence of HH:MM" logic (unit-tested).
- `src-tauri/src/alarm.rs` - the engine: scheduler thread + ring thread + state.
- `src/` - SolidJS + TypeScript UI that talks to the Rust commands.

## License

[MIT](./LICENSE) © 2026 adi
