import { cn } from "@/lib/cn";
import { useMotionAllowed } from "@/hooks/useMotionAllowed";
import type { StatusTone } from "@/lib/status";

const DOT_COLORS: Record<StatusTone, string> = {
  neutral: "bg-idle",
  muted: "bg-ink-muted",
  success: "bg-ok",
  failure: "bg-bad",
  warn: "bg-warn",
  running: "bg-live",
};

interface StatusPulseProps {
  tone: StatusTone;
  pulse: boolean;
  className?: string;
}

/**
 * Status dot with an optional expanding ring.
 *
 * The ring only renders when motion is allowed *and* the state is transient
 * (`in_progress`, `action_required`), so a wall of finished repos stays still.
 */
export function StatusPulse({ tone, pulse, className }: StatusPulseProps) {
  const motion = useMotionAllowed();
  const animate = pulse && motion;

  return (
    <span
      className={cn("relative inline-flex size-2 shrink-0", className)}
      aria-hidden="true"
    >
      {animate ? (
        <span
          className={cn(
            "absolute inset-0 rounded-full animate-pulse-ring",
            DOT_COLORS[tone],
          )}
        />
      ) : null}
      <span
        className={cn("relative inline-flex size-2 rounded-full", DOT_COLORS[tone])}
      />
    </span>
  );
}
