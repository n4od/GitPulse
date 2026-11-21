import { useQuery } from "@tanstack/react-query";
import type { SystemMetrics } from "@/lib/bindings/SystemMetrics";
import { cn } from "@/lib/cn";
import { call } from "@/lib/ipc";
import { useLiveStore } from "@/stores/live";

/**
 * The bottom strip: what the poller is doing, and what the cache holds.
 *
 * Both halves are deliberately terse. It answers "is it still working?" and
 * "how much is stored?" without competing with the cards for attention.
 */
export function StatusBar() {
  const poller = useLiveStore((store) => store.poller);
  const paused = useLiveStore((store) => store.paused);

  const metrics = useQuery({
    queryKey: ["system-metrics"],
    queryFn: () => call<SystemMetrics>("system_metrics"),
    staleTime: 30_000,
    refetchInterval: 30_000,
  });

  const state = paused
    ? "Paused by a rate limit"
    : poller?.lastError
      ? `Last poll error: ${poller.lastError}`
      : poller
        ? `Watching ${poller.monitored} ${poller.monitored === 1 ? "repository" : "repositories"} · every ${poller.targetIntervalSecs}s`
        : "Waiting for the first poll";

  const dot = paused ? "bg-bad" : poller?.lastError ? "bg-warn" : "bg-ok";

  return (
    <footer className="flex h-11 shrink-0 items-center gap-2 border-t border-line bg-chrome px-8 text-[11.5px] text-ink-muted">
      <span className={cn("size-[7px] rounded-full", dot)} aria-hidden="true" />
      <span className="truncate">{state}</span>

      {metrics.data ? (
        <span className="ml-auto whitespace-nowrap tabular-nums">
          Cache: {metrics.data.monitoredRepos}{" "}
          {metrics.data.monitoredRepos === 1 ? "repository" : "repositories"} ·{" "}
          {Number(metrics.data.cachedRuns)} runs · {metrics.data.dbSizeMb.toFixed(1)} MB
        </span>
      ) : null}
    </footer>
  );
}
