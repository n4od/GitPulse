import type { ReactNode } from "react";
import { GitPulseMark } from "@/components/shell/GitPulseMark";

interface TitleBarProps {
  actions?: ReactNode;
}

/**
 * Window chrome. The native title bar is kept (`decorations: true`), so this is
 * an in-app header rather than a drag region.
 */
export function TitleBar({ actions }: TitleBarProps) {
  return (
    <header className="flex h-14 shrink-0 items-center gap-2.5 border-b border-line bg-chrome px-5">
      <GitPulseMark className="text-accent" />
      <span className="text-sm font-semibold tracking-tight text-ink">GitPulse</span>
      {actions ? <div className="ml-auto flex items-center gap-3">{actions}</div> : null}
    </header>
  );
}
