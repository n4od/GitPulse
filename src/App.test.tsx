import { render, screen, waitFor } from "@testing-library/react";
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
  invoke.mockImplementation((command: string) =>
    // The dashboard is only mounted when signed in, and it needs a list, not
    // an auth payload.
    command === "repos_list" ? Promise.resolve([]) : Promise.resolve(state),
  );
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

    expect(screen.getByText("Pulse")).toBeInTheDocument();
    expect(
      await screen.findByRole("heading", { name: /sign in to github/i }),
    ).toBeInTheDocument();
  });

  it("shows the signed-in identity once a session arrives", () => {
    seed(SIGNED_IN);
    render(<App />);

    expect(screen.getByText("octocat")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /sign out/i })).toBeInTheDocument();
  });

  it("signs out through the backend when the button is used", async () => {
    seed(SIGNED_IN);
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: /sign out/i }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("auth_sign_out", undefined);
    });
  });

  it("keeps the dev gallery out of the default route", () => {
    seed(SIGNED_OUT);
    render(<App />);

    expect(
      screen.queryByRole("heading", { name: /component gallery/i }),
    ).not.toBeInTheDocument();
  });
});
