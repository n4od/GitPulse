import type { ReactNode } from "react";
import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, ExternalLink, Play, RotateCcw, Square, X } from "lucide-react";
import type { Deployment } from "@/lib/bindings/Deployment";
import type { Job } from "@/lib/bindings/Job";
import type { LogDownload } from "@/lib/bindings/LogDownload";
import type { PullRequest } from "@/lib/bindings/PullRequest";
import type { RepoCard } from "@/lib/bindings/RepoCard";
import type { WorkflowRun } from "@/lib/bindings/WorkflowRun";
import { Badge } from "@/components/primitives/Badge";
import { Button } from "@/components/primitives/Button";
import { Spinner } from "@/components/primitives/Spinner";
import { StatusBadge } from "@/components/status/StatusBadge";
import { LogViewer } from "@/components/runs/LogViewer";
import { DispatchDialog } from "@/components/runs/DispatchDialog";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { latestRun, relativeTime, shortSha } from "@/lib/repo-selectors";
import { REPOS_QUERY_KEY } from "@/hooks/useRepos";
import { optimisticallySetRunStatus } from "@/lib/run-selectors";
import { toast } from "@/stores/toasts";

type Tab = "runs" | "deployments" | "pulls";

/**
 * The run inspector.
 *
 * Runs come from the card that is already in the cache, so opening this costs
 * nothing until you pick a run; jobs and logs are fetched on demand because
 * they are only interesting while someone is looking.
 */
export function RunDetail({ card, onClose }: { card: RepoCard; onClose: () => void }) {
  const queryClient = useQueryClient();
  const [tab, setTab] = useState<Tab>("runs");
  const [runId, setRunId] = useState<number | null>(null);
  const [jobId, setJobId] = useState<number | null>(null);
  const [dispatching, setDispatching] = useState(false);

  const selectedRun = card.recentRuns.find((run) => run.id === runId) ?? null;

  const jobs = useQuery({
    queryKey: ["jobs", runId],
    queryFn: () => call<Job[]>("runs_jobs", { runId }),
    enabled: runId !== null,
  });

  const log = useQuery({
    queryKey: ["log", jobId],
    queryFn: () => call<LogDownload>("job_log", { jobId }),
    enabled: jobId !== null,
  });

  const deployments = useQuery({
    queryKey: ["deployments", card.repo.id],
    queryFn: () => call<Deployment[]>("repos_deployments", { repoId: card.repo.id }),
    enabled: tab === "deployments",
  });

  const pulls = useQuery({
    queryKey: ["pulls", card.repo.id],
    queryFn: () => call<PullRequest[]>("repos_pulls", { repoId: card.repo.id }),
    enabled: tab === "pulls",
  });

  /**
   * Optimistic with rollback: the status changes immediately, and the previous
   * card is restored if GitHub refuses.
   */
  const runAction = useMutation({
    mutationFn: async (input: { command: string; run: WorkflowRun }) =>
      call(input.command, { runId: input.run.id }),
    onMutate: (input) => {
      const previous = queryClient.getQueryData<RepoCard[]>(REPOS_QUERY_KEY);

      queryClient.setQueryData<RepoCard[]>(REPOS_QUERY_KEY, (cards) =>
        cards?.map((entry) =>
          entry.repo.id === card.repo.id
            ? optimisticallySetRunStatus(
                entry,
                input.run.id,
                input.command === "runs_cancel" ? "completed" : "queued",
              )
            : entry,
        ),
      );

      return { previous };
    },
    onError: (cause: unknown, _input, context) => {
      if (context?.previous) {
        queryClient.setQueryData(REPOS_QUERY_KEY, context.previous);
      }

      toast("error", "GitHub refused that action", toPulseError(cause).message);
    },
    onSuccess: (_result, input) => {
      toast(
        "success",
        input.command === "runs_cancel" ? "Cancellation requested" : "Re-run queued",
      );
      void queryClient.invalidateQueries({ queryKey: ["jobs", input.run.id] });
    },
  });

  function openOnGitHub(url: string | null): void {
    if (!url) return;
    void call("system_open_url", { url }).catch(() => {
      toast("error", "Could not open the browser");
    });
  }

  async function copyRunUrl(run: WorkflowRun): Promise<void> {
    if (!run.htmlUrl) return;

    try {
      await navigator.clipboard.writeText(run.htmlUrl);
      toast("success", "Run URL copied");
    } catch {
      toast("error", "Could not copy the URL");
    }
  }

  const newest = latestRun(card);

  return (
    <aside className="flex h-full min-h-0 w-full flex-col border-l border-line bg-surface">
      <header className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-2">
        <span className="truncate text-[13px] font-medium">{card.repo.fullName}</span>
        {newest ? (
          <StatusBadge status={newest.status} conclusion={newest.conclusion} />
        ) : null}
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto"
          aria-label="Close the run inspector"
          onClick={onClose}
        >
          <X className="size-3.5" />
        </Button>
      </header>

      <nav className="flex shrink-0 gap-1 border-b border-line px-2 py-1">
        {(["runs", "deployments", "pulls"] as const).map((entry) => (
          <Button
            key={entry}
            size="sm"
            variant={tab === entry ? "secondary" : "ghost"}
            aria-pressed={tab === entry}
            onClick={() => {
              setTab(entry);
            }}
          >
            {entry === "pulls"
              ? "Pull requests"
              : entry[0]?.toUpperCase() + entry.slice(1)}
          </Button>
        ))}
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto"
          leadingIcon={<Play className="size-3.5" />}
          onClick={() => {
            setDispatching(true);
          }}
        >
          Dispatch
        </Button>
      </nav>

      {tab === "runs" ? (
        <div className="flex min-h-0 flex-1">
          <ul className="w-64 shrink-0 divide-y divide-line overflow-y-auto border-r border-line">
            {card.recentRuns.length === 0 ? (
              <li className="p-3 text-xs text-ink-muted">No runs cached yet.</li>
            ) : (
              card.recentRuns.map((run) => (
                <li key={run.id}>
                  <button
                    type="button"
                    onClick={() => {
                      setRunId(run.id);
                      setJobId(null);
                    }}
                    className={`w-full px-3 py-2 text-left transition-colors hover:bg-raised ${
                      runId === run.id ? "bg-raised" : ""
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <span className="truncate text-xs text-ink">
                        {run.workflowName ?? "workflow"}
                      </span>
                      <StatusBadge
                        status={run.status}
                        conclusion={run.conclusion}
                        className="ml-auto"
                      />
                    </div>
                    <div className="mt-1 flex items-center gap-1.5 text-[11px] text-ink-muted">
                      <span className="font-mono">{run.branch ?? "—"}</span>
                      <span>{shortSha(run.headSha)}</span>
                      <span className="ml-auto">{relativeTime(run.updatedAt)}</span>
                    </div>
                  </button>
                </li>
              ))
            )}
          </ul>

          <div className="flex min-h-0 flex-1 flex-col">
            {selectedRun ? (
              <>
                <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-line px-3 py-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    leadingIcon={<RotateCcw className="size-3.5" />}
                    onClick={() => {
                      runAction.mutate({
                        command: "runs_rerun_failed",
                        run: selectedRun,
                      });
                    }}
                  >
                    Re-run failed
                  </Button>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      runAction.mutate({ command: "runs_rerun", run: selectedRun });
                    }}
                  >
                    Re-run all
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    leadingIcon={<Square className="size-3.5" />}
                    onClick={() => {
                      runAction.mutate({ command: "runs_cancel", run: selectedRun });
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label="Copy the run URL"
                    onClick={() => {
                      void copyRunUrl(selectedRun);
                    }}
                  >
                    <Copy className="size-3.5" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label="Open the run on GitHub"
                    onClick={() => {
                      openOnGitHub(selectedRun.htmlUrl);
                    }}
                  >
                    <ExternalLink className="size-3.5" />
                  </Button>
                </div>

                {jobId === null ? (
                  <div className="min-h-0 flex-1 overflow-y-auto p-3">
                    {jobs.isPending ? (
                      <div className="flex items-center gap-2 text-xs text-ink-muted">
                        <Spinner className="size-3.5" /> Loading jobs…
                      </div>
                    ) : jobs.isError ? (
                      <p className="text-xs text-bad">
                        {toPulseError(jobs.error).message}
                      </p>
                    ) : (jobs.data ?? []).length === 0 ? (
                      <p className="text-xs text-ink-muted">
                        No jobs reported for this run.
                      </p>
                    ) : (
                      <ul className="space-y-1">
                        {(jobs.data ?? []).map((job) => (
                          <li key={job.id}>
                            <button
                              type="button"
                              onClick={() => {
                                setJobId(job.id);
                              }}
                              className="flex w-full items-center gap-2 rounded-control border border-line bg-raised px-3 py-2 text-left hover:border-line-strong"
                            >
                              <span className="truncate text-xs text-ink">
                                {job.name}
                              </span>
                              <StatusBadge
                                status={job.status}
                                conclusion={job.conclusion}
                                className="ml-auto"
                              />
                            </button>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                ) : (
                  <div className="flex min-h-0 flex-1 flex-col">
                    <div className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-1.5">
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => {
                          setJobId(null);
                        }}
                      >
                        ← Jobs
                      </Button>
                      <span className="truncate text-xs text-ink-soft">
                        {(jobs.data ?? []).find((job) => job.id === jobId)?.name ?? "log"}
                      </span>
                    </div>

                    {log.isPending ? (
                      <div className="flex items-center gap-2 p-3 text-xs text-ink-muted">
                        <Spinner className="size-3.5" /> Downloading log…
                      </div>
                    ) : log.isError ? (
                      <p className="p-3 text-xs text-bad">
                        {toPulseError(log.error).message}
                      </p>
                    ) : log.data ? (
                      <LogViewer download={log.data} />
                    ) : null}
                  </div>
                )}
              </>
            ) : (
              <p className="p-3 text-xs text-ink-muted">Pick a run to see its jobs.</p>
            )}
          </div>
        </div>
      ) : tab === "deployments" ? (
        <Panel
          pending={deployments.isPending}
          error={deployments.error}
          empty="No deployments found."
          count={(deployments.data ?? []).length}
        >
          {(deployments.data ?? []).map((deployment) => (
            <li
              key={deployment.id}
              className="flex items-center gap-2 rounded-control border border-line bg-raised px-3 py-2"
            >
              <span className="text-xs text-ink">{deployment.environment}</span>
              <span className="font-mono text-[11px] text-ink-muted">
                {deployment.ref ?? "—"}
              </span>
              <Badge
                tone={
                  deployment.state === "success"
                    ? "success"
                    : deployment.state === "failure"
                      ? "failure"
                      : "neutral"
                }
                className="ml-auto"
              >
                {deployment.state ?? "unknown"}
              </Badge>
            </li>
          ))}
        </Panel>
      ) : (
        <Panel
          pending={pulls.isPending}
          error={pulls.error}
          empty="No open pull requests."
          count={(pulls.data ?? []).length}
        >
          {(pulls.data ?? []).map((pull) => (
            <li key={pull.id}>
              <button
                type="button"
                onClick={() => {
                  openOnGitHub(pull.htmlUrl);
                }}
                className="flex w-full items-center gap-2 rounded-control border border-line bg-raised px-3 py-2 text-left hover:border-line-strong"
              >
                <span className="font-mono text-[11px] text-ink-muted">
                  #{pull.number}
                </span>
                <span className="truncate text-xs text-ink">{pull.title}</span>
                {pull.isDraft ? <Badge tone="muted">draft</Badge> : null}
                <span className="ml-auto shrink-0 text-[11px] text-ink-muted">
                  {relativeTime(pull.updatedAt)}
                </span>
              </button>
            </li>
          ))}
        </Panel>
      )}

      {dispatching ? (
        <DispatchDialog
          repoId={card.repo.id}
          defaultRef={card.repo.defaultBranch ?? "main"}
          onClose={() => {
            setDispatching(false);
          }}
        />
      ) : null}
    </aside>
  );
}

interface PanelProps {
  pending: boolean;
  error: unknown;
  empty: string;
  count: number;
  children: ReactNode;
}

function Panel({ pending, error, empty, count, children }: PanelProps) {
  if (pending) {
    return (
      <div className="flex items-center gap-2 p-3 text-xs text-ink-muted">
        <Spinner className="size-3.5" /> Loading…
      </div>
    );
  }

  if (error) {
    return <p className="p-3 text-xs text-bad">{toPulseError(error).message}</p>;
  }

  if (count === 0) {
    return <p className="p-3 text-xs text-ink-muted">{empty}</p>;
  }

  return <ul className="min-h-0 flex-1 space-y-1 overflow-y-auto p-3">{children}</ul>;
}
