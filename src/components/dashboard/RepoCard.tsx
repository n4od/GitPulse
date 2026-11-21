import { memo } from "react";
import { Copy, ExternalLink, BellOff, RefreshCw } from "lucide-react";
import type { RepoCard as RepoCardData } from "@/lib/bindings/RepoCard";
import { Badge } from "@/components/primitives/Badge";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody } from "@/components/primitives/Card";
import { StatusBadge } from "@/components/status/StatusBadge";
import { RunSparkline } from "@/components/dashboard/RunSparkline";
import { latestRun, relativeTime, runDuration, shortSha } from "@/lib/repo-selectors";
import { call } from "@/lib/ipc";
import { toast } from "@/stores/toasts";

export interface RepoCardProps {
  card: RepoCardData;
  onRefresh: (repoId: number) => void;
  onToggleMonitored: (repoId: number, monitored: boolean) => void;
  onOpen: (repoId: number) => void;
}

/**
 * A repository card.
 *
 * Memoised on purpose: a `run://updated` event replaces one card object in the
 * cache, and only the card whose reference changed should re-render.
 */
export const RepoCard = memo(function RepoCard({
  card,
  onRefresh,
  onToggleMonitored,
  onOpen,
}: RepoCardProps) {
  const { repo } = card;
  const run = latestRun(card);

  function openOnGitHub(): void {
    const target = run?.htmlUrl ?? `https://github.com/${repo.fullName}/actions`;
    void call("system_open_url", { url: target }).catch(() => {
      toast("error", "Could not open the browser");
    });
  }

  async function copyRunUrl(): Promise<void> {
    if (!run?.htmlUrl) {
      toast("info", "No run to link to yet");
      return;
    }

    try {
      await navigator.clipboard.writeText(run.htmlUrl);
      toast("success", "Run URL copied");
    } catch {
      toast("error", "Could not copy the URL");
    }
  }

  return (
    <Card className="group flex h-full flex-col transition-colors hover:border-line-strong">
      <CardBody className="flex flex-1 flex-col px-4 pb-3 pt-4">
        <div className="flex items-start gap-2">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={() => {
                  onOpen(repo.id);
                }}
                className="truncate text-[14px] font-semibold tracking-tight text-ink hover:text-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent"
                title={`Inspect ${repo.fullName}`}
              >
                {repo.fullName}
              </button>
              {repo.isPrivate ? (
                <Badge tone="muted" className="px-1.5 py-0 text-[10px]">
                  private
                </Badge>
              ) : null}
            </div>
            {repo.monitored ? null : (
              <span className="text-[11px] text-ink-muted">muted</span>
            )}
          </div>

          {run ? (
            <StatusBadge
              status={run.status}
              conclusion={run.conclusion}
              indicator={false}
              className="mt-0.5 border-transparent px-2.5 py-[3px] text-[9.5px] font-bold uppercase tracking-[0.06em]"
            />
          ) : (
            <Badge tone="muted" className="text-[10px]">
              No runs
            </Badge>
          )}
        </div>

        {run ? (
          <div className="mt-1 truncate text-[11.5px] text-ink-muted">
            <span className="font-medium text-ink-soft">
              {run.workflowName ?? "workflow"}
            </span>
            <span> · {run.branch ?? "—"}</span>
            {run.headSha ? (
              <span className="font-mono"> · {shortSha(run.headSha)}</span>
            ) : null}
          </div>
        ) : (
          <p className="mt-1 text-[11.5px] text-ink-muted">
            GitPulse has not seen a run for this repository yet.
          </p>
        )}

        {card.failingRuns > 0 ? (
          <div className="mt-1 text-[11px] text-bad">
            {card.failingRuns} failing {card.failingRuns === 1 ? "run" : "runs"}
          </div>
        ) : null}

        {card.recentRuns.length > 1 ? (
          <RunSparkline runs={card.recentRuns} className="mt-2" />
        ) : (
          <div className="flex-1" />
        )}

        <div className="mt-auto flex items-center gap-2 pt-2 text-[11px] text-ink-muted">
          <span className="tabular-nums">
            {run ? relativeTime(run.updatedAt) : "never"}
          </span>
          {run ? (
            <>
              <span
                className="size-[3px] rounded-full bg-line-strong"
                aria-hidden="true"
              />
              <span className="tabular-nums">{runDuration(run)}</span>
            </>
          ) : null}
          {run ? (
            <>
              <span
                className="size-[3px] rounded-full bg-line-strong"
                aria-hidden="true"
              />
              <span className="tabular-nums">
                {card.recentRuns.length} {card.recentRuns.length === 1 ? "run" : "runs"}
              </span>
            </>
          ) : null}

          <div className="ml-auto flex items-center gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
            <Button
              size="sm"
              variant="ghost"
              className="size-7 px-0"
              aria-label={`Refresh ${repo.fullName}`}
              onClick={() => {
                onRefresh(repo.id);
              }}
            >
              <RefreshCw className="size-3.5" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="size-7 px-0"
              aria-label={`Copy the run URL for ${repo.fullName}`}
              onClick={() => {
                void copyRunUrl();
              }}
            >
              <Copy className="size-3.5" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="size-7 px-0"
              aria-label={`Mute ${repo.fullName}`}
              onClick={() => {
                onToggleMonitored(repo.id, !repo.monitored);
              }}
            >
              <BellOff className="size-3.5" />
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="size-7 px-0"
              aria-label={`Open ${repo.fullName} on GitHub`}
              onClick={openOnGitHub}
            >
              <ExternalLink className="size-3.5" />
            </Button>
          </div>
        </div>
      </CardBody>
    </Card>
  );
});
