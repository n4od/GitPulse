import type { ReactNode } from "react";

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 px-6 py-12 text-center">
      {icon ? (
        <div className="grid size-10 place-items-center rounded-card border border-line bg-raised text-ink-muted">
          {icon}
        </div>
      ) : null}
      <div className="space-y-1">
        <h2 className="text-sm font-semibold text-ink">{title}</h2>
        <p className="max-w-sm text-xs leading-5 text-ink-soft">{description}</p>
      </div>
      {action ? <div className="pt-1">{action}</div> : null}
    </div>
  );
}
