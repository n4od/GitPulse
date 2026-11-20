import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import type { AuthState } from "@/lib/bindings/AuthState";
import { useAuthStore, useSession, useSigningIn } from "@/stores/auth";

const SIGNED_OUT: AuthState = { phase: "signedOut" };

const POLING: AuthState = {
  phase: "polling",
  userCode: "ABCD-1234",
  verificationUri: "https://github.com/login/device",
  expiresAt: "2026-01-01T00:00:00Z",
};

const AUTHORIZED: AuthState = {
  phase: "authorized",
  session: {
    login: "octocat",
    avatarUrl: null,
    displayName: null,
    expiresAt: null,
    scopes: ["repo"],
    canRefresh: true,
  },
};

function setState(state: AuthState) {
  act(() => {
    useAuthStore.setState({ state });
  });
}

describe("auth store", () => {
  beforeEach(() => {
    useAuthStore.setState({ state: SIGNED_OUT, ready: false });
  });

  it("exposes no session when signed out", () => {
    const { result } = renderHook(() => useSession());
    expect(result.current).toBeNull();
  });

  it("exposes the session when authorized", () => {
    setState(AUTHORIZED);
    const { result } = renderHook(() => useSession());
    expect(result.current?.login).toBe("octocat");
  });

  it("reports a flow in progress only for the transient phases", () => {
    const { result } = renderHook(() => useSigningIn());
    expect(result.current).toBe(false);

    setState(POLING);
    expect(result.current).toBe(true);

    setState(AUTHORIZED);
    expect(result.current).toBe(false);
  });

  it("starts unready and becomes ready once the bridge settles", () => {
    const { result } = renderHook(() => useAuthStore((store) => store.ready));
    expect(result.current).toBe(false);

    act(() => {
      useAuthStore.getState().setReady();
    });

    expect(result.current).toBe(true);
  });
});
