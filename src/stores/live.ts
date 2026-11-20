import { create } from "zustand";
import type { PollerStatus } from "@/lib/bindings/PollerStatus";
import type { RateLimitSnapshot } from "@/lib/bindings/RateLimitSnapshot";

interface LiveStore {
  poller: PollerStatus | null;
  rate: RateLimitSnapshot | null;
  /** Whether a secondary limit is currently in force. */
  paused: boolean;
  setPoller: (status: PollerStatus) => void;
  setRate: (rate: RateLimitSnapshot) => void;
}

/**
 * Background telemetry: what the poller is doing and how much of the hourly
 * quota is left. Kept out of the query cache because it is pushed, never
 * fetched, and nothing should try to revalidate it.
 *
 * `paused` is computed here, when the event arrives, rather than during render:
 * reading the clock while rendering is impure and would make the component's
 * output depend on when React happened to re-render it.
 */
export const useLiveStore = create<LiveStore>((set) => ({
  poller: null,
  rate: null,
  paused: false,
  setPoller: (poller) => set({ poller }),
  setRate: (rate) =>
    set({
      rate,
      paused: rate.pausedUntil !== null && Date.parse(rate.pausedUntil) > Date.now(),
    }),
}));
