import type { WorkflowRun } from "@/lib/bindings/WorkflowRun";
import { cn } from "@/lib/cn";
import { mapRunStatus } from "@/lib/status";

/**
 * Sparkline stroke per status tone. Uses the tokens rather than hex so the
 * chart and the badge can never disagree about what "passing" looks like.
 */
const TONE_STROKE: Record<string, string> = {
  success: "var(--color-ok)",
  failure: "var(--color-bad)",
  warn: "var(--color-warn)",
  running: "var(--color-live)",
  muted: "var(--color-idle)",
  neutral: "var(--color-idle)",
};

const MAX_POINTS = 10;
const VIEW_WIDTH = 100;
const VIEW_HEIGHT = 40;
const PADDING = 5;

/**
 * One point per recent run, oldest on the left, height by how long it took.
 *
 * A line rather than bars: a rising line means builds are getting slower, which
 * is the thing worth noticing at a glance. The stroke takes the newest run's
 * colour, so a repository that has started failing turns red immediately.
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
  const ordered = runs.slice(0, MAX_POINTS).reverse();
  const durations = ordered.map(durationSeconds);
  const longest = Math.max(...durations, 1);

  const step = ordered.length > 1 ? VIEW_WIDTH / (ordered.length - 1) : VIEW_WIDTH / 2;

  const points = ordered
    .map((_, index) => {
      const x = ordered.length > 1 ? index * step : VIEW_WIDTH / 2;
      const ratio = (durations[index] ?? 0) / longest;
      const y = VIEW_HEIGHT - PADDING - ratio * (VIEW_HEIGHT - PADDING * 2);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  const newest = ordered[ordered.length - 1];
  const tone = newest ? mapRunStatus(newest.status, newest.conclusion).tone : "neutral";
  const stroke = TONE_STROKE[tone] ?? "var(--color-idle)";

  const lastY =
    VIEW_HEIGHT -
    PADDING -
    ((durations[durations.length - 1] ?? 0) / longest) * (VIEW_HEIGHT - PADDING * 2);

  return (
    <svg
      viewBox={`0 0 ${VIEW_WIDTH} ${VIEW_HEIGHT}`}
      preserveAspectRatio="none"
      className={cn("h-10 w-full", className)}
      role="img"
      aria-label={`Last ${ordered.length} runs, oldest to newest`}
    >
      <polyline
        points={points}
        fill="none"
        stroke={stroke}
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
      {newest ? (
        <circle
          cx={VIEW_WIDTH}
          cy={lastY}
          r="2.5"
          fill={stroke}
          vectorEffect="non-scaling-stroke"
        />
      ) : null}
    </svg>
  );
}

function durationSeconds(run: WorkflowRun): number {
  const started = run.runStartedAt ? Date.parse(run.runStartedAt) : Number.NaN;
  if (Number.isNaN(started)) return 0;

  const finished = run.conclusion ? Date.parse(run.updatedAt) : Date.now();
  return Math.max(0, Math.round((finished - started) / 1000));
}
