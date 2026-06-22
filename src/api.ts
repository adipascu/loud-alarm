import { invoke } from "@tauri-apps/api/core";
import type { SoundKind, Status } from "./types";

export function armAlarm(
  hour: number,
  minute: number,
  sound: SoundKind,
  forceVolume: boolean,
  volumeLevel: number,
): Promise<Status> {
  return invoke<Status>("arm_alarm", {
    hour,
    minute,
    sound,
    forceVolume,
    volumeLevel,
  });
}

export function disarmAlarm(): Promise<Status> {
  return invoke<Status>("disarm_alarm");
}

export function stopRinging(): Promise<Status> {
  return invoke<Status>("stop_ringing");
}

export function getStatus(): Promise<Status> {
  return invoke<Status>("get_status");
}

export function previewSound(
  sound: SoundKind,
  volumeLevel: number,
): Promise<void> {
  return invoke<void>("preview_sound", { sound, volumeLevel });
}
