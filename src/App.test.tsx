import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Status } from "./types";

const disarmed: Status = {
  armed: false,
  ringing: false,
  hour: 0,
  minute: 0,
  secondsRemaining: 0,
  forceVolume: true,
  volumeLevel: 100,
  sound: "Siren",
};

const armed: Status = {
  ...disarmed,
  armed: true,
  hour: 7,
  minute: 0,
  secondsRemaining: 3600,
};

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import App from "./App";

beforeEach(() => {
  invoke.mockReset();
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === "get_status") return disarmed;
    if (cmd === "arm_alarm") return armed;
    return undefined;
  });
});

describe("App", () => {
  it("renders the setup screen with the default 07:00 time", async () => {
    render(() => <App />);
    expect(screen.getByText("Loud Alarm")).toBeInTheDocument();
    expect(screen.getByText("beta")).toBeInTheDocument();
    expect(screen.getByDisplayValue("07:00")).toBeInTheDocument();
    expect(screen.getByText("Arm alarm")).toBeInTheDocument();
  });

  it("arms the alarm with the chosen time and volume", async () => {
    render(() => <App />);
    fireEvent.click(screen.getByText("Arm alarm"));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("arm_alarm", {
        hour: 7,
        minute: 0,
        sound: "Siren",
        forceVolume: true,
        volumeLevel: 100,
      });
    });
    await waitFor(() =>
      expect(screen.getByText("Alarm armed for")).toBeInTheDocument(),
    );
  });

  it("previews a sound at the chosen volume without arming", async () => {
    render(() => <App />);
    fireEvent.click(screen.getByText("Preview"));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("preview_sound", {
        sound: "Siren",
        volumeLevel: 100,
      }),
    );
  });
});
