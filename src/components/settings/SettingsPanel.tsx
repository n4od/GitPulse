import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Database, LogOut, Webhook } from "lucide-react";
import type { NativeSettings } from "@/lib/bindings/NativeSettings";
import type { SystemMetrics } from "@/lib/bindings/SystemMetrics";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody, CardHeader } from "@/components/primitives/Card";
import { Spinner } from "@/components/primitives/Spinner";
import { useLiveStore } from "@/stores/live";
import { useSettingsStore } from "@/stores/settings";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { signOut } from "@/lib/auth-commands";
import { toast } from "@/stores/toasts";

interface SettingsPanelProps {
  onClose: () => void;
  onOpenWebhooks: () => void;
}

export function SettingsPanel({ onClose, onOpenWebhooks }: SettingsPanelProps) {
  const settings = useSettingsStore((store) => store.settings);
  const saving = useSettingsStore((store) => store.saving);
  const load = useSettingsStore((store) => store.load);
  const update = useSettingsStore((store) => store.update);
  const poller = useLiveStore((store) => store.poller);

  // A draft, not state mirrored from props: mirroring needs an effect, and an
  // effect that sets state on every settings change is a render loop waiting
  // to happen. `null` means "showing what the backend says".
  const [quietDraft, setQuietDraft] = useState<string | null>(null);
  const quietValue = quietDraft ?? settings?.quietHours ?? "";

  useEffect(() => {
    void load();
  }, [load]);

  const metrics = useQuery({
    queryKey: ["metrics"],
    queryFn: () => call<SystemMetrics>("system_metrics"),
    refetchInterval: 10_000,
  });

  async function clearCache(): Promise<void> {
    try {
      await call("system_clear_cache");
      await metrics.refetch();
      toast("success", "Cache cleared", "Repositories will be refetched shortly.");
    } catch (cause) {
      toast("error", "Could not clear the cache", toPulseError(cause).message);
    }
  }

  function toggle(key: keyof NativeSettings, value: boolean): void {
    void update({ [key]: value });
  }

  return (
    <div className="absolute inset-0 z-40 grid place-items-center bg-surface/80 p-6">
      <Card className="max-h-full w-full max-w-lg overflow-y-auto">
        <CardHeader>
          <span className="text-[13px] font-medium">Settings</span>
          {saving ? <Spinner className="size-3.5 text-ink-muted" /> : null}
          <Button size="sm" variant="ghost" className="ml-auto" onClick={onClose}>
            Close
          </Button>
        </CardHeader>

        <CardBody className="space-y-4">
          {settings === null ? (
            <div className="flex items-center gap-2 text-xs text-ink-muted">
              <Spinner className="size-3.5" /> Loading…
            </div>
          ) : (
            <>
              <fieldset className="space-y-2">
                <legend className="mb-1 text-[11px] uppercase tracking-wider text-ink-muted">
                  Notifications
                </legend>

                <label className="flex items-center gap-2 text-xs text-ink">
                  <input
                    type="checkbox"
                    checked={settings.notificationsEnabled}
                    onChange={(event) => {
                      toggle("notificationsEnabled", event.target.checked);
                    }}
                  />
                  Notify me when a build fails
                </label>

                <label className="flex items-center gap-2 text-xs text-ink">
                  <span className="w-40 shrink-0 text-ink-soft">Quiet hours</span>
                  <input
                    value={quietValue}
                    onChange={(event) => {
                      setQuietDraft(event.target.value);
                    }}
                    onBlur={() => {
                      const value = quietValue.trim();
                      setQuietDraft(null);
                      void update({ quietHours: value === "" ? null : value });
                    }}
                    placeholder="22-07"
                    aria-label="Quiet hours, for example 22-07"
                    className="h-7 w-24 rounded-control border border-line bg-raised px-2 font-mono text-[11px] text-ink focus-visible:outline-none"
                  />
                  <span className="text-[11px] text-ink-muted">empty = never quiet</span>
                </label>
              </fieldset>

              <fieldset className="space-y-2 border-t border-line pt-3">
                <legend className="mb-1 text-[11px] uppercase tracking-wider text-ink-muted">
                  Window
                </legend>

                <label className="flex items-center gap-2 text-xs text-ink">
                  <input
                    type="checkbox"
                    checked={settings.closeToTray}
                    onChange={(event) => {
                      toggle("closeToTray", event.target.checked);
                    }}
                  />
                  Keep running when the window is closed
                </label>

                <label className="flex items-center gap-2 text-xs text-ink">
                  <input
                    type="checkbox"
                    checked={settings.autostart}
                    onChange={(event) => {
                      toggle("autostart", event.target.checked);
                    }}
                  />
                  Start Pulse when I sign in
                </label>
              </fieldset>

              <fieldset className="space-y-2 border-t border-line pt-3">
                <legend className="mb-1 text-[11px] uppercase tracking-wider text-ink-muted">
                  Polling
                </legend>

                <p className="text-[11px] leading-4 text-ink-muted">
                  {poller
                    ? `${poller.monitored} repositories, revisited about every ${Math.round(
                        poller.targetIntervalSecs / 60,
                      )} minutes. The interval is derived from GitHub's hourly limit, so a large
                      watchlist is polled less often rather than hitting the ceiling.`
                    : "Waiting for the first poll."}
                </p>
              </fieldset>

              <fieldset className="space-y-2 border-t border-line pt-3">
                <legend className="mb-1 text-[11px] uppercase tracking-wider text-ink-muted">
                  Diagnostics
                </legend>

                {metrics.isPending ? (
                  <Spinner className="size-3.5 text-ink-muted" />
                ) : metrics.isError ? (
                  <p className="text-[11px] text-bad">
                    {toPulseError(metrics.error).message}
                  </p>
                ) : metrics.data ? (
                  <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-[11px]">
                    <Metric
                      label="Cache size"
                      value={`${metrics.data.dbSizeMb.toFixed(1)} MB`}
                    />
                    <Metric
                      label="Repositories"
                      value={String(metrics.data.monitoredRepos)}
                    />
                    <Metric label="Cached runs" value={String(metrics.data.cachedRuns)} />
                    <Metric
                      label="Webhook deliveries"
                      value={String(metrics.data.deliveries)}
                    />
                    <Metric
                      label="Requests left"
                      value={String(metrics.data.budgetAvailable)}
                    />
                    <Metric
                      label="Uptime"
                      value={`${Math.round(metrics.data.uptimeSecs / 60)} min`}
                    />
                  </dl>
                ) : null}

                <div className="flex gap-2 pt-1">
                  <Button
                    size="sm"
                    variant="secondary"
                    leadingIcon={<Database className="size-3.5" />}
                    onClick={() => {
                      void clearCache();
                    }}
                  >
                    Clear cache
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    leadingIcon={<Webhook className="size-3.5" />}
                    onClick={onOpenWebhooks}
                  >
                    Webhooks
                  </Button>
                </div>
              </fieldset>

              <div className="border-t border-line pt-3">
                <Button
                  size="sm"
                  variant="ghost"
                  leadingIcon={<LogOut className="size-3.5" />}
                  onClick={() => {
                    void signOut();
                    onClose();
                  }}
                >
                  Sign out
                </Button>
              </div>
            </>
          )}
        </CardBody>
      </Card>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <>
      <dt className="text-ink-muted">{label}</dt>
      <dd className="text-right font-mono tabular-nums text-ink-soft">{value}</dd>
    </>
  );
}
