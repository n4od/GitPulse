import { Gauge } from "lucide-react";
import { Tooltip } from "@/components/primitives/Tooltip";
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
      : null;

  const tone =
    fraction === null
      ? "text-ink-muted"
      : fraction > 0.5
        ? "text-ok"
        : fraction > 0.15
          ? "text-warn"
          : "text-bad";

  return (
    <Tooltip
      side="bottom"
      label={
        poller
          ? `${poller.monitored} repositories · every ${poller.targetIntervalSecs}s · ${poller.budgetAvailable} local requests left`
          : "Waiting for the first poll"
      }
    >
      <div className="flex items-center gap-1.5 rounded-control border border-line px-2 py-1">
        <Gauge className={`size-3.5 ${tone}`} aria-hidden="true" />
        <span className="font-mono text-[11px] tabular-nums text-ink-soft">
          {remaining === null ? "—" : `${remaining}/${limit ?? "?"}`}
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
