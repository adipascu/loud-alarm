import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import {
  armAlarm,
  disarmAlarm,
  getStatus,
  previewSound,
  stopRinging,
} from "./api";
import { formatCountdown, formatTime, parseTime } from "./lib/format";
import { SOUND_KINDS, type SoundKind, type Status } from "./types";
import "./App.css";

const POLL_MS = 500;

function App() {
  const [time, setTime] = createSignal("07:00");
  const [sound, setSound] = createSignal<SoundKind>("Siren");
  const [forceVolume, setForceVolume] = createSignal(true);
  const [volumeLevel, setVolumeLevel] = createSignal(100);
  const [status, setStatus] = createSignal<Status | null>(null);
  const [error, setError] = createSignal("");

  async function refresh() {
    try {
      setStatus(await getStatus());
    } catch (e) {
      setError(String(e));
    }
  }

  onMount(() => {
    void refresh();
    const handle = setInterval(() => void refresh(), POLL_MS);
    onCleanup(() => clearInterval(handle));
  });

  async function onArm() {
    setError("");
    const parsed = parseTime(time());
    if (!parsed) {
      setError("Enter a valid time as HH:MM");
      return;
    }
    try {
      setStatus(
        await armAlarm(
          parsed.hour,
          parsed.minute,
          sound(),
          forceVolume(),
          volumeLevel(),
        ),
      );
    } catch (e) {
      setError(String(e));
    }
  }

  const onDisarm = async () => setStatus(await disarmAlarm());
  const onStop = async () => setStatus(await stopRinging());

  const armed = () => status()?.armed ?? false;
  const ringing = () => status()?.ringing ?? false;

  return (
    <main class="app">
      <header class="header">
        <h1>
          <span class="bell">⏰</span> Loud Alarm
        </h1>
        <span class="badge">beta</span>
      </header>

      <Show
        when={ringing()}
        fallback={
          <section class="card">
            <Show
              when={armed()}
              fallback={
                <>
                  <label class="field">
                    <span>Ring at</span>
                    <input
                      class="time-input"
                      type="time"
                      value={time()}
                      onInput={(e) => setTime(e.currentTarget.value)}
                    />
                  </label>

                  <label class="field">
                    <span>Sound</span>
                    <div class="sound-row">
                      <select
                        class="select"
                        value={sound()}
                        onChange={(e) =>
                          setSound(e.currentTarget.value as SoundKind)
                        }
                      >
                        <For each={SOUND_KINDS}>
                          {(kind) => <option value={kind}>{kind}</option>}
                        </For>
                      </select>
                      <button
                        type="button"
                        class="btn btn-ghost"
                        onClick={() =>
                          void previewSound(sound(), volumeLevel())
                        }
                      >
                        Preview
                      </button>
                    </div>
                  </label>

                  <label class="field">
                    <span>Volume: {volumeLevel()}%</span>
                    <input
                      class="slider"
                      type="range"
                      min="0"
                      max="100"
                      value={volumeLevel()}
                      onInput={(e) =>
                        setVolumeLevel(Number(e.currentTarget.value))
                      }
                    />
                  </label>

                  <label class="toggle">
                    <input
                      type="checkbox"
                      checked={forceVolume()}
                      onChange={(e) => setForceVolume(e.currentTarget.checked)}
                    />
                    <span>
                      Override system volume so it rings even when muted
                      (restored afterwards)
                    </span>
                  </label>

                  <p class="hint">
                    Plays as media audio, so Focus / Do Not Disturb will not
                    silence it. With override off, the volume above is applied
                    in-app and your system volume is never touched.
                  </p>

                  <button
                    type="button"
                    class="btn btn-arm"
                    onClick={() => void onArm()}
                  >
                    Arm alarm
                  </button>
                </>
              }
            >
              <div class="armed">
                <p class="armed-label">Alarm armed for</p>
                <p class="armed-time">
                  {formatTime(status()!.hour, status()!.minute)}
                </p>
                <p class="countdown">
                  rings in {formatCountdown(status()!.secondsRemaining)}
                </p>
                <button
                  type="button"
                  class="btn btn-ghost"
                  onClick={() => void onDisarm()}
                >
                  Disarm
                </button>
              </div>
            </Show>

            <Show when={error()}>
              <p class="error">{error()}</p>
            </Show>
          </section>
        }
      >
        <section class="ringing" role="alert">
          <div class="ringing-pulse">RINGING</div>
          <p class="ringing-time">
            {formatTime(status()!.hour, status()!.minute)}
          </p>
          <button
            type="button"
            class="btn btn-stop"
            onClick={() => void onStop()}
          >
            Stop
          </button>
        </section>
      </Show>
    </main>
  );
}

export default App;
