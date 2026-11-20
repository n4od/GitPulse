import { useEffect, useState } from "react";
import { Copy, ExternalLink, X } from "lucide-react";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody, CardHeader } from "@/components/primitives/Card";
import { Spinner } from "@/components/primitives/Spinner";
import { toast } from "@/stores/toasts";

interface DeviceCodeCardProps {
  userCode: string;
  verificationUri: string;
  /** RFC 3339 timestamp from GitHub. */
  expiresAt: string;
  polling: boolean;
  onCancel: () => void;
  onReopen: () => void;
}

function secondsUntil(iso: string): number {
  const target = Date.parse(iso);
  if (Number.isNaN(target)) return 0;
  return Math.max(0, Math.ceil((target - Date.now()) / 1000));
}

function formatRemaining(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${minutes}:${String(seconds % 60).padStart(2, "0")}`;
}

export function DeviceCodeCard({
  userCode,
  verificationUri,
  expiresAt,
  polling,
  onCancel,
  onReopen,
}: DeviceCodeCardProps) {
  const remaining = useCountdown(expiresAt);

  async function copyCode(): Promise<void> {
    try {
      await navigator.clipboard.writeText(userCode);
      toast("success", "Code copied");
    } catch {
      toast("error", "Could not copy the code", "Select it and copy manually.");
    }
  }

  return (
    <Card className="w-full max-w-md">
      <CardHeader>
        <span className="text-[13px] font-medium">Finish signing in</span>
        <span
          className="ml-auto font-mono text-[11px] tabular-nums text-ink-muted"
          title="Time until this code stops working"
        >
          {formatRemaining(remaining)}
        </span>
      </CardHeader>

      <CardBody className="space-y-4">
        <ol className="space-y-1 text-xs leading-5 text-ink-soft">
          <li>1. Your browser has opened GitHub&apos;s device page.</li>
          <li>2. Enter this code:</li>
        </ol>

        <div className="flex items-center gap-2">
          <code className="selectable flex-1 rounded-control border border-line-strong bg-surface px-3 py-2 text-center font-mono text-lg tracking-[0.3em] text-ink">
            {userCode}
          </code>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void copyCode()}
            leadingIcon={<Copy className="size-3.5" />}
            aria-label="Copy the device code"
          >
            Copy
          </Button>
        </div>

        <div className="flex items-center gap-2 text-xs text-ink-muted">
          {polling ? (
            <>
              <Spinner className="size-3.5" />
              <span>Waiting for you to approve…</span>
            </>
          ) : (
            <span>Waiting for GitHub…</span>
          )}
        </div>

        <div className="flex items-center gap-2 pt-1">
          <Button
            size="sm"
            variant="secondary"
            onClick={onReopen}
            leadingIcon={<ExternalLink className="size-3.5" />}
          >
            Reopen GitHub
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={onCancel}
            leadingIcon={<X className="size-3.5" />}
          >
            Cancel
          </Button>
        </div>

        <p className="selectable text-[11px] leading-4 text-ink-muted">
          {verificationUri}
        </p>
      </CardBody>
    </Card>
  );
}

/** Ticks once a second while the code is alive. */
function useCountdown(expiresAt: string): number {
  const [remaining, setRemaining] = useState(() => secondsUntil(expiresAt));

  useEffect(() => {
    // No synchronous setState here: the initial value came from the state
    // initialiser, and the first tick corrects any drift within a second.
    const timer = window.setInterval(() => setRemaining(secondsUntil(expiresAt)), 1000);
    return () => window.clearInterval(timer);
  }, [expiresAt]);

  return remaining;
}
