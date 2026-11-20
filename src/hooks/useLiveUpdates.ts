import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import type { PollerStatus } from "@/lib/bindings/PollerStatus";
import type { RateLimitSnapshot } from "@/lib/bindings/RateLimitSnapshot";
import type { RepoCard } from "@/lib/bindings/RepoCard";
import { applyCardUpdate } from "@/lib/repo-selectors";
import { isTauriRuntime } from "@/lib/tauri";
import { REPOS_QUERY_KEY } from "@/hooks/useRepos";
import { useLiveStore } from "@/stores/live";

export const RUNS_UPDATED_EVENT = "run://updated";
export const POLLER_STATUS_EVENT = "poller://status";
export const RATE_LIMIT_EVENT = "ratelimit://updated";

/**
 * Subscribes to the backend's push events. Mount exactly once, at the app root.
 *
 * The important one is `run://updated`: it carries a whole recomputed card, so
 * the cache is patched in place rather than invalidated. Invalidating would
 * refetch all 250 cards because one repository finished a build.
 */
export function useLiveUpdates(): void {
  const queryClient = useQueryClient();
  const setPoller = useLiveStore((store) => store.setPoller);
  const setRate = useLiveStore((store) => store.setRate);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let active = true;
    const stops: Array<() => void> = [];

    const register = async (): Promise<void> => {
      const subscriptions: Array<[string, (payload: unknown) => void]> = [
        [
          RUNS_UPDATED_EVENT,
          (payload) => {
            const card = payload as RepoCard;
            queryClient.setQueryData<RepoCard[]>(REPOS_QUERY_KEY, (cards) =>
              cards ? applyCardUpdate(cards, card) : cards,
            );
          },
        ],
        [POLLER_STATUS_EVENT, (payload) => setPoller(payload as PollerStatus)],
        [RATE_LIMIT_EVENT, (payload) => setRate(payload as RateLimitSnapshot)],
      ];

      for (const [event, handler] of subscriptions) {
        try {
          const stop = await listen(event, (message) => {
            if (active) handler(message.payload);
          });

          if (active) {
            stops.push(stop);
          } else {
            stop();
          }
        } catch (error) {
          console.error(`could not subscribe to ${event}`, error);
        }
      }
    };

    void register();

    return () => {
      active = false;
      for (const stop of stops) stop();
    };
  }, [queryClient, setPoller, setRate]);
}
