/** Format a positive duration in seconds as a compact "4h 09m 12s" string. */
export function formatCountdown(totalSeconds: number): string {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");

  if (h > 0) return `${h}h ${pad(m)}m ${pad(s)}s`;
  if (m > 0) return `${m}m ${pad(s)}s`;
  return `${s}s`;
}

/** Format an hour/minute pair as 24-hour "HH:MM". */
export function formatTime(hour: number, minute: number): string {
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${pad(hour)}:${pad(minute)}`;
}

/** Parse an "HH:MM" string into validated hour/minute, or null if malformed. */
export function parseTime(
  value: string,
): { hour: number; minute: number } | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (hour > 23 || minute > 59) return null;
  return { hour, minute };
}
