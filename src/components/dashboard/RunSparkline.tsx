import type { WorkflowRun } from "@/lib/bindings/WorkflowRun";
import { cn } from "@/lib/cn";
import { mapRunStatus } from "@/lib/status";

const TONE_BARS: Record<string, string> = {
  success: "bg-ok",
  failure: "bg-bad",
  warn: "bg-warn",
  running: "bg-live",
  muted: "bg-idle",
  neutral: "bg-line-strong",
};

const MAX_BARS = 10;
const MIN_HEIGHT_PERCENT = 22;

/**
 * Bar per recent run, oldest on the left, height by duration.
 *
 * The bar colour uses the same status vocabulary as the badge, so a repository
 * whose builds are getting slower is visible before you open anything.
 */
export function RunSparkline({
  runs,
  className,
}: {
  runs: WorkflowRun[];
  className?: string;
}) {
  if (runs.length === 0) return null;

  // `recentRuns` is newest first; a sparkline reads left to right.
  const ordered = runs.slice(0, MAX_BARS).reverse();
  const durations = ordered.map(durationSeconds);
  const longest = Math.max(...durations, 1);

  return (
    <div
      className={cn("flex h-4 items-end gap-0.5", className)}
      role="img"
      aria-label={`Last ${ordered.length} runs, oldest to newest`}
    >
      {ordered.map((run, index) => {
        const seconds = durations[index] ?? 0;
        const height = Math.max(MIN_HEIGHT_PERCENT, (seconds / longest) * 100);
        const tone = mapRunStatus(run.status, run.conclusion).tone;

        return (
          <span
            key={run.id}
            title={`${mapRunStatus(run.status, run.conclusion).label} · ${seconds}s`}
            className={cn("block w-1 rounded-sm", TONE_BARS[tone] ?? "bg-line-strong")}
            style={{ height: `${height}%` }}
          />
        );
      })}
    </div>
  );
}

function durationSeconds(run: WorkflowRun): number {
  const started = run.runStartedAt ? Date.parse(run.runStartedAt) : Number.NaN;
  if (Number.isNaN(started)) return 0;

  const finished = run.conclusion ? Date.parse(run.updatedAt) : Date.now();
  return Math.max(0, Math.round((finished - started) / 1000));
}
