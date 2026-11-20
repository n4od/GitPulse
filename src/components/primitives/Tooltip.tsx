import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

interface TooltipProps {
  label: string;
  side?: "top" | "bottom";
  className?: string;
  children: ReactNode;
}

/**
 * CSS-only tooltip: no state, no portal, no layout thrash.
 *
 * It shows on hover and on `:focus-within`, so keyboard users get it for free.
 * The bubble is `aria-hidden` on purpose — it duplicates the trigger's own
 * accessible name, and announcing it twice is worse than not announcing it.
 * Use `aria-label` on the trigger for the authoritative text.
 */
export function Tooltip({ label, side = "top", className, children }: TooltipProps) {
  return (
    <span className={cn("group relative inline-flex", className)}>
      {children}
      <span
        role="tooltip"
        aria-hidden="true"
        className={cn(
          "pointer-events-none absolute left-1/2 z-40 -translate-x-1/2 whitespace-nowrap",
          "rounded-control border border-line-strong bg-overlay px-2 py-1",
          "text-[11px] font-medium text-ink shadow-lg",
          "opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-within:opacity-100",
          side === "top" ? "bottom-[calc(100%+6px)]" : "top-[calc(100%+6px)]",
        )}
      >
        {label}
      </span>
    </span>
  );
}
