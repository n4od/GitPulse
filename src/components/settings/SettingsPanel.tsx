import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Database, RefreshCw } from "lucide-react";
import type { NativeSettings } from "@/lib/bindings/NativeSettings";
import type { SystemMetrics } from "@/lib/bindings/SystemMetrics";
import type { UpdateProgress } from "@/lib/bindings/UpdateProgress";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody, CardHeader } from "@/components/primitives/Card";
import { Spinner } from "@/components/primitives/Spinner";
import { useLiveStore } from "@/stores/live";
import { useSettingsStore } from "@/stores/settings";
import { useUpdateStore } from "@/stores/update";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/cn";
import { toPulseError } from "@/lib/errors";
import { toast } from "@/stores/toasts";

interface SettingsPanelProps {
  onClose: () => void;
}

/** One decimal place, the way the cache size is shown. */
function megabytes(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * The install in flight.
 *
 * This exists because pressing "Install and restart" used to remove the button
 * and change one line of small text, and then the app was gone. It is the last
 * thing anyone sees before the window is taken away, so it says what is
 * happening and how far along it is.
 *
 * The bar is omitted while the size is unknown rather than shown empty or full:
 * a bar that never moves is indistinguishable from a hung download, and a full
 * one claims an upgrade that has not happened.
 */
function InstallProgress({
  version,
  progress,
}: {
  version: string | null;
  progress: UpdateProgress | null;
}) {
  const total = progress?.total ?? null;
  const downloaded = progress?.downloaded ?? 0;
  const stage = progress?.stage ?? "preparing";

  const percent =
    total !== null && total > 0
      ? Math.max(0, Math.min(100, Math.round((downloaded / total) * 100)))
      : null;

  // An install re-reads the manifest before it fetches anything, so the first
  // thing it does can fail with nothing downloaded. "Preparing" is the honest
  // word for that moment; "downloading" is not.
  const verb =
    stage === "installing"
      ? "Installing"
      : stage === "preparing"
        ? "Preparing to install"
        : "Downloading";
  const target = version ? `GitPulse ${version}` : "the update";

  return (
    <div className="space-y-1.5">
      <div
        role="status"
        className="flex items-center gap-2 text-[11px] leading-4 text-ink-soft"
      >
        <Spinner className="size-3.5" />
        <span>
          {verb} {target}
          {stage === "installing"
            ? " - GitPulse will restart by itself."
            : percent === null
              ? "..."
              : ` - ${percent}%`}
        </span>
      </div>

      {percent === null ? null : (
        <>
          <div
            role="progressbar"
            aria-label={`${verb} ${target}`}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
            className="h-1.5 w-full overflow-hidden rounded-full bg-overlay"
          >
            <div
              className={cn(
                "h-full rounded-full bg-accent transition-[width] duration-200",
                stage === "installing" && "bg-ok",
              )}
              style={{ width: `${percent}%` }}
            />
          </div>
          {total === null ? null : (
            <p className="text-[10px] tabular-nums text-ink-muted">
              {megabytes(downloaded)} of {megabytes(total)}
            </p>
          )}
        </>
      )}
    </div>
  );
}

export function SettingsPanel({ onClose }: SettingsPanelProps) {
  const settings = useSettingsStore((store) => store.settings);
  const saving = useSettingsStore((store) => store.saving);
  const load = useSettingsStore((store) => store.load);
  const update = useSettingsStore((store) => store.update);
  const poller = useLiveStore((store) => store.poller);

  const version = useUpdateStore((store) => store.version);
  const updatePhase = useUpdateStore((store) => store.phase);
  const available = useUpdateStore((store) => store.available);
  const progress = useUpdateStore((store) => store.progress);
  const updateError = useUpdateStore((store) => store.error);
  const updateRetryable = useUpdateStore((store) => store.retryable);
  const loadVersion = useUpdateStore((store) => store.loadVersion);
  const checkForUpdates = useUpdateStore((store) => store.check);
  const installUpdate = useUpdateStore((store) => store.install);

  // A draft, not state mirrored from props: mirroring needs an effect, and an
  // effect that sets state on every settings change is a render loop waiting
  // to happen. `null` means "showing what the backend says".
  const [quietDraft, setQuietDraft] = useState<string | null>(null);
  const quietValue = quietDraft ?? settings?.quietHours ?? "";

  useEffect(() => {
    void load();
    void loadVersion();
  }, [load, loadVersion]);

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
                    className="h-7 w-24 rounded-control bg-field px-2 font-mono text-[11px] text-ink focus-visible:outline-none"
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
                  Start GitPulse when I sign in
                </label>
              </fieldset>

              <fieldset className="space-y-2 border-t border-line pt-3">
                <legend className="mb-1 text-[11px] uppercase tracking-wider text-ink-muted">
                  Updates
                </legend>

                <label className="flex items-center gap-2 text-xs text-ink">
                  <input
                    type="checkbox"
                    checked={settings.checkUpdatesOnLaunch}
                    onChange={(event) => {
                      toggle("checkUpdatesOnLaunch", event.target.checked);
                    }}
                  />
                  Check for updates when GitPulse starts
                </label>

                <p className="text-[11px] leading-4 text-ink-muted">
                  Version{" "}
                  <span className="font-mono text-ink-soft">{version ?? "-"}</span>
                  {updatePhase === "checking"
                    ? " - checking..."
                    : updatePhase === "current"
                      ? " - up to date."
                      : updatePhase === "available" && available
                        ? ` - ${available.version} is available.`
                        : null}
                </p>

                {updatePhase === "installing" ? (
                  <InstallProgress
                    version={available?.version ?? null}
                    progress={progress}
                  />
                ) : null}

                {updateError !== null ? (
                  <p className="text-[11px] leading-4 text-bad">
                    {updateError}
                    {updateRetryable ? " Try again in a moment." : null}
                  </p>
                ) : null}

                <div className="flex flex-wrap gap-2 pt-1">
                  <Button
                    size="sm"
                    variant="secondary"
                    loading={updatePhase === "checking"}
                    leadingIcon={<RefreshCw className="size-3.5" />}
                    className="bg-field hover:bg-line-strong active:bg-field"
                    disabled={updatePhase === "checking" || updatePhase === "installing"}
                    onClick={() => {
                      void checkForUpdates();
                    }}
                  >
                    Check for updates
                  </Button>

                  {updatePhase === "available" || updatePhase === "installing" ? (
                    // Kept on screen while installing, in a busy state, rather
                    // than swapped out: the button vanishing under the cursor is
                    // most of what made the click look like it did nothing.
                    <Button
                      size="sm"
                      variant="primary"
                      loading={updatePhase === "installing"}
                      disabled={updatePhase === "installing"}
                      onClick={() => {
                        void installUpdate();
                      }}
                    >
                      {available
                        ? `Install ${available.version} and restart`
                        : "Install and restart"}
                    </Button>
                  ) : null}
                </div>

                {updatePhase === "available" && available?.notes ? (
                  <details className="text-[11px] text-ink-muted">
                    <summary className="cursor-pointer">
                      What is new in {available.version}
                    </summary>
                    <p className="mt-1 max-h-40 overflow-y-auto whitespace-pre-line leading-4">
                      {available.notes}
                    </p>
                  </details>
                ) : null}
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
                    className="bg-field hover:bg-line-strong active:bg-field"
                    onClick={() => {
                      void clearCache();
                    }}
                  >
                    Clear cache
                  </Button>
                </div>
              </fieldset>
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
