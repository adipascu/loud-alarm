export type SoundKind = "Siren" | "Beep" | "Chirp";

export const SOUND_KINDS: readonly SoundKind[] = ["Siren", "Beep", "Chirp"];

/** Mirrors the Rust `alarm::Status` struct (serialized camelCase). */
export interface Status {
  armed: boolean;
  ringing: boolean;
  hour: number;
  minute: number;
  secondsRemaining: number;
  forceVolume: boolean;
  sound: SoundKind;
}
