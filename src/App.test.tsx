import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => undefined)),
}));

import App from "@/App";
import type { AuthState } from "@/lib/bindings/AuthState";
import { useAuthStore } from "@/stores/auth";

const SIGNED_OUT: AuthState = { phase: "signedOut" };

const SIGNED_IN: AuthState = {
  phase: "authorized",
  session: {
    login: "octocat",
    avatarUrl: null,
    displayName: null,
    expiresAt: null,
    scopes: [],
    canRefresh: false,
  },
};

/**
 * Seeds both the store and the bridge's initial fetch. Seeding only the store
 * would be overwritten by the `auth_get_state` round-trip, which is exactly
 * what happens on a real launch.
 */
function seed(state: AuthState): void {
  invoke.mockImplementation((command: string) => {
    // The dashboard and the settings panel are only mounted when signed in,
    // and they need their own payloads, not the auth one.
    switch (command) {
      case "repos_list":
        return Promise.resolve([]);
      case "auth_accounts":
        return Promise.resolve(["octocat"]);
      case "native_get_settings":
        return Promise.resolve({
          notificationsEnabled: true,
          quietHours: null,
          autostart: false,
          closeToTray: true,
        });
      case "system_metrics":
        return Promise.resolve({
          dbSizeMb: 0,
          monitoredRepos: 0,
          cachedRuns: 0,
          budgetAvailable: 30,
          uptimeSecs: 1,
        });
      default:
        return Promise.resolve(state);
    }
  });

  useAuthStore.setState({ state, ready: true });
}

describe("App shell", () => {
  beforeEach(() => {
    invoke.mockReset();
    useAuthStore.setState({ state: SIGNED_OUT, ready: false });
    window.location.hash = "";
  });

  it("renders the title bar and the signed-out landing route", async () => {
    seed(SIGNED_OUT);
    render(<App />);

    expect(screen.getByText("GitPulse")).toBeInTheDocument();
    expect(
      await screen.findByRole("heading", { name: /sign in to github/i }),
    ).toBeInTheDocument();
  });

  it("shows the signed-in identity once a session arrives", () => {
    seed(SIGNED_IN);
    render(<App />);

    expect(screen.getByText("octocat")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^settings$/i })).toBeInTheDocument();
  });

  it("opens settings from the title bar", async () => {
    seed(SIGNED_IN);
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: /^settings$/i }));

    expect(await screen.findByText(/notify me when a build fails/i)).toBeInTheDocument();
  });

  it("keeps the dev gallery out of the default route", () => {
    seed(SIGNED_OUT);
    render(<App />);

    expect(
      screen.queryByRole("heading", { name: /component gallery/i }),
    ).not.toBeInTheDocument();
  });
});
