import { describe, expect, it } from "vitest";
import { formatCountdown, formatTime, parseTime } from "./format";

describe("formatCountdown", () => {
  it("formats hours, minutes and seconds with zero-padding", () => {
    expect(formatCountdown(4 * 3600 + 9 * 60 + 12)).toBe("4h 09m 12s");
  });

  it("drops the hours segment under an hour", () => {
    expect(formatCountdown(5 * 60 + 3)).toBe("5m 03s");
  });

  it("shows only seconds under a minute", () => {
    expect(formatCountdown(42)).toBe("42s");
  });

  it("clamps negative input to zero", () => {
    expect(formatCountdown(-100)).toBe("0s");
  });
});

describe("formatTime", () => {
  it("zero-pads hours and minutes", () => {
    expect(formatTime(17, 0)).toBe("17:00");
    expect(formatTime(9, 5)).toBe("09:05");
  });
});

describe("parseTime", () => {
  it("parses a valid HH:MM string", () => {
    expect(parseTime("17:00")).toEqual({ hour: 17, minute: 0 });
    expect(parseTime("09:05")).toEqual({ hour: 9, minute: 5 });
  });

  it("rejects out-of-range and malformed values", () => {
    expect(parseTime("24:00")).toBeNull();
    expect(parseTime("12:60")).toBeNull();
    expect(parseTime("nope")).toBeNull();
    expect(parseTime("")).toBeNull();
  });
});
