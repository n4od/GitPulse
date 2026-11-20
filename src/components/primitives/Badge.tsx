import type { ComponentPropsWithRef, ReactNode } from "react";
import { cn } from "@/lib/cn";
import type { StatusTone } from "@/lib/status";

const TONES: Record<StatusTone | "accent", string> = {
  neutral: "border-line bg-raised text-ink-soft",
  muted: "border-line bg-transparent text-ink-muted",
  success: "border-ok/30 bg-ok/10 text-ok",
  failure: "border-bad/30 bg-bad/10 text-bad",
  warn: "border-warn/30 bg-warn/10 text-warn",
  running: "border-live/30 bg-live/10 text-live",
  accent: "border-accent/30 bg-accent/10 text-accent",
};

export interface BadgeProps extends ComponentPropsWithRef<"span"> {
  tone?: StatusTone | "accent";
  /** Optional leading indicator, e.g. a pulsing dot. */
  indicator?: ReactNode;
}

export function Badge({
  tone = "neutral",
  indicator,
  className,
  children,
  ...rest
}: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 whitespace-nowrap rounded-full border px-2 py-0.5",
        "text-[11px] font-medium leading-4",
        TONES[tone],
        className,
      )}
      {...rest}
    >
      {indicator}
      {children}
    </span>
  );
}
