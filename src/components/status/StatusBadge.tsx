import { Badge } from "@/components/primitives/Badge";
import { StatusPulse } from "@/components/status/StatusPulse";
import { mapRunStatus } from "@/lib/status";

interface StatusBadgeProps {
  status?: string | null;
  conclusion?: string | null;
  className?: string;
}

/**
 * The only component allowed to render a workflow status. Routing every badge
 * through `mapRunStatus` is what keeps the dashboard, tray and notifications
 * agreeing with each other.
 */
export function StatusBadge({
  status = null,
  conclusion = null,
  className,
}: StatusBadgeProps) {
  const presentation = mapRunStatus(status, conclusion);

  return (
    <Badge
      tone={presentation.tone}
      indicator={<StatusPulse tone={presentation.tone} pulse={presentation.pulse} />}
      className={className}
      title={`${status ?? "unknown"} / ${conclusion ?? "pending"}`}
    >
      {presentation.label}
    </Badge>
  );
}
