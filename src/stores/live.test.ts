import { beforeEach, describe, expect, it } from "vitest";
import type { RateLimitSnapshot } from "@/lib/bindings/RateLimitSnapshot";
import { useLiveStore } from "@/stores/live";

const RATE: RateLimitSnapshot = {
  limit: 5000,
  remaining: 4321,
  used: 679,
  resetAt: "2026-11-20T13:00:00Z",
  pausedUntil: null,
  localAvailable: 12,
};

describe("live store", () => {
  beforeEach(() => {
    useLiveStore.setState({ poller: null, rate: null, paused: false });
  });

  it("starts empty", () => {
    expect(useLiveStore.getState().rate).toBeNull();
    expect(useLiveStore.getState().poller).toBeNull();
    expect(useLiveStore.getState().paused).toBe(false);
  });

  it("stores the rate limit snapshot", () => {
    useLiveStore.getState().setRate(RATE);
    expect(useLiveStore.getState().rate?.remaining).toBe(4321);
  });

  it("is not paused when the pause is in the past", () => {
    useLiveStore.getState().setRate({ ...RATE, pausedUntil: "2020-01-01T00:00:00Z" });
    expect(useLiveStore.getState().paused).toBe(false);
  });

  it("is paused when the pause is in the future", () => {
    const future = new Date(Date.now() + 60_000).toISOString();
    useLiveStore.getState().setRate({ ...RATE, pausedUntil: future });
    expect(useLiveStore.getState().paused).toBe(true);
  });

  it("stores poller status", () => {
    useLiveStore.getState().setPoller({
      monitored: 250,
      targetIntervalSecs: 257,
      budgetAvailable: 30,
      lastError: null,
    });

    expect(useLiveStore.getState().poller?.monitored).toBe(250);
  });
});
