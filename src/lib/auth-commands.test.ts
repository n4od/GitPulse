import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import {
  cancelSignIn,
  openVerificationUri,
  retrySignIn,
  signOut,
  startSignIn,
} from "@/lib/auth-commands";
import type { AuthState } from "@/lib/bindings/AuthState";
import { useToastStore } from "@/stores/toasts";

const AUTHORIZED: AuthState = {
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

function toasts() {
  return useToastStore.getState().toasts;
}

describe("auth commands", () => {
  beforeEach(() => {
    invoke.mockReset();
    useToastStore.setState({ toasts: [] });
  });

  it("starts the device flow", async () => {
    invoke.mockResolvedValue(undefined);
    await startSignIn();
    expect(invoke).toHaveBeenCalledWith("auth_start_device_flow", undefined);
  });

  it("reports a failure to start as an error toast", async () => {
    invoke.mockRejectedValue({
      code: "config",
      message: "no GitHub client id is configured",
      retryable: false,
    });

    await startSignIn();

    expect(toasts()).toHaveLength(1);
    expect(toasts()[0]?.tone).toBe("error");
    expect(toasts()[0]?.description).toBe("no GitHub client id is configured");
  });

  it("cancels a flow in progress", async () => {
    invoke.mockResolvedValue(true);
    await cancelSignIn();
    expect(invoke).toHaveBeenCalledWith("auth_cancel_device_flow", undefined);
  });

  it("signs out and confirms it", async () => {
    invoke.mockResolvedValue(undefined);
    await signOut();

    expect(invoke).toHaveBeenCalledWith("auth_sign_out", undefined);
    expect(toasts()[0]).toMatchObject({ tone: "info", title: "Signed out" });
  });

  it("reports a failure to sign out", async () => {
    invoke.mockRejectedValue(new Error("keychain is locked"));
    await signOut();

    expect(toasts()[0]?.tone).toBe("error");
    expect(toasts()[0]?.description).toBe("keychain is locked");
  });

  it("does not restart the flow when the session is still valid", async () => {
    invoke.mockResolvedValueOnce(AUTHORIZED).mockResolvedValue(undefined);

    await retrySignIn();

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("auth_refresh_session", undefined);
  });

  it("restarts the flow when the session could not be recovered", async () => {
    invoke
      .mockResolvedValueOnce({ phase: "signedOut" } satisfies AuthState)
      .mockResolvedValue(undefined);

    await retrySignIn();

    expect(invoke).toHaveBeenNthCalledWith(1, "auth_refresh_session", undefined);
    expect(invoke).toHaveBeenNthCalledWith(2, "auth_start_device_flow", undefined);
  });

  it("reports a failure to refresh", async () => {
    invoke.mockRejectedValue(new Error("network request failed"));

    await retrySignIn();

    expect(toasts()[0]?.tone).toBe("error");
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it("opens the verification page through the validated system command", () => {
    invoke.mockResolvedValue(undefined);

    openVerificationUri("https://github.com/login/device");

    expect(invoke).toHaveBeenCalledWith("system_open_url", {
      url: "https://github.com/login/device",
    });
  });
});
