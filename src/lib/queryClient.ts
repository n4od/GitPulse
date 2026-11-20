import { QueryClient } from "@tanstack/react-query";

/**
 * One client for the whole app.
 *
 * `staleTime: Infinity` and no `refetchInterval` are deliberate: freshness
 * arrives over events, not polling. A query refetches when it has no data, and
 * the live bridge patches it thereafter — see `useLiveUpdates`.
 */
export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: Infinity,
        gcTime: 5 * 60_000,
        retry: 1,
        refetchOnWindowFocus: false,
        refetchOnReconnect: false,
      },
    },
  });
}
