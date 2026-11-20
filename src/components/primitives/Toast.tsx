import { useEffect } from "react";
import { CheckCircle2, Info, X, XCircle } from "lucide-react";
import { cn } from "@/lib/cn";
import { useToastStore, type Toast, type ToastTone } from "@/stores/toasts";

const TONE_STYLES: Record<ToastTone, string> = {
  info: "border-line-strong text-ink",
  success: "border-ok/40 text-ink",
  error: "border-bad/40 text-ink",
};

const TONE_ICONS: Record<ToastTone, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  error: XCircle,
};

const TONE_ICON_COLORS: Record<ToastTone, string> = {
  info: "text-accent",
  success: "text-ok",
  error: "text-bad",
};

const AUTO_DISMISS_MS = 5_000;

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: (id: string) => void;
}) {
  useEffect(() => {
    const timer = window.setTimeout(() => onDismiss(toast.id), AUTO_DISMISS_MS);
    return () => window.clearTimeout(timer);
  }, [toast.id, onDismiss]);

  const Icon = TONE_ICONS[toast.tone];

  return (
    <div
      className={cn(
        "pointer-events-auto flex items-start gap-2.5 rounded-card border bg-overlay p-3 shadow-xl",
        TONE_STYLES[toast.tone],
      )}
    >
      <Icon
        className={cn("mt-0.5 size-4 shrink-0", TONE_ICON_COLORS[toast.tone])}
        aria-hidden="true"
      />
      <div className="min-w-0 flex-1">
        <p className="text-[13px] font-medium leading-5">{toast.title}</p>
        {toast.description ? (
          <p className="mt-0.5 text-xs leading-4 text-ink-soft">{toast.description}</p>
        ) : null}
      </div>
      <button
        type="button"
        onClick={() => onDismiss(toast.id)}
        aria-label="Dismiss notification"
        className="rounded p-1 text-ink-muted transition-colors hover:bg-raised hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
      >
        <X className="size-3.5" aria-hidden="true" />
      </button>
    </div>
  );
}

/**
 * Renders the toast queue in a polite live region.
 *
 * `aria-live="polite"` means a screen reader announces a failure without
 * interrupting whatever the user is typing.
 */
export function Toaster() {
  const toasts = useToastStore((state) => state.toasts);
  const dismiss = useToastStore((state) => state.dismiss);

  return (
    <div
      aria-live="polite"
      aria-atomic="false"
      className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-80 flex-col gap-2"
    >
      {toasts.map((item) => (
        <ToastItem key={item.id} toast={item} onDismiss={dismiss} />
      ))}
    </div>
  );
}
