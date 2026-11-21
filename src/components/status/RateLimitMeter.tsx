import { Tooltip } from "@/components/primitives/Tooltip";
import { cn } from "@/lib/cn";
import { useLiveStore } from "@/stores/live";

/**
 * How much of the hourly quota is left, and what the poller is doing.
 *
 * The numbers come from GitHub's own response headers, so this is the
 * authoritative picture rather than the local estimate.
 */
export function RateLimitMeter() {
  const rate = useLiveStore((store) => store.rate);
  const poller = useLiveStore((store) => store.poller);
  const paused = useLiveStore((store) => store.paused);

  if (!rate && !poller) return null;

  const remaining = rate?.remaining ?? null;
  const limit = rate?.limit ?? null;

  const fraction =
    remaining !== null && limit !== null && limit > 0
      ? Math.max(0, Math.min(1, remaining / limit))
      : 0;

  const barTone = fraction > 0.5 ? "bg-ok" : fraction > 0.15 ? "bg-warn" : "bg-bad";

  return (
    <Tooltip
      side="bottom"
      label={
        poller
          ? `${poller.monitored} repositories · every ${poller.targetIntervalSecs}s · ${poller.budgetAvailable} local requests left`
          : "Waiting for the first poll"
      }
    >
      <div className="flex items-center gap-2">
        <span className="whitespace-nowrap text-[11px] text-ink-muted">GitHub API</span>
        <div className="h-1.5 w-24 overflow-hidden rounded-full bg-overlay">
          <div
            className={cn("h-full rounded-full transition-[width] duration-500", barTone)}
            style={{ width: `${Math.round(fraction * 100)}%` }}
          />
        </div>
        <span className="font-mono text-[11px] tabular-nums text-ink-soft">
          {remaining === null
            ? "—"
            : `${remaining.toLocaleString()} / ${(limit ?? 0).toLocaleString()}`}
        </span>
        {paused ? (
          <span
            className="text-[10px] font-medium text-warn"
            title="Paused by a rate limit"
          >
            paused
          </span>
        ) : null}
      </div>
    </Tooltip>
  );
}
