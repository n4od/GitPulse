import { Profiler, useCallback, useMemo, useRef, useState } from "react";
import type { ProfilerOnRenderCallback } from "react";
import type { RepoCard as RepoCardData } from "@/lib/bindings/RepoCard";
import { DashboardGrid } from "@/components/dashboard/DashboardGrid";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody } from "@/components/primitives/Card";
import { applyCardUpdate, filterCards } from "@/lib/repo-selectors";
import { navigate } from "@/lib/router";

/**
 * Dev-only performance bench.
 *
 * The roadmap's budget is "p95 React commit under 16 ms with 250 repositories".
 * That is a claim about a real browser, so this measures it in one: a React
 * `Profiler` accumulates every commit's actual duration, and the numbers are
 * shown directly.
 *
 * It is excluded from production builds by the same `import.meta.env.DEV` guard
 * as the component gallery.
 */

const CARD_COUNT = 250;

type Stats = { count: number; p50: number; p95: number; max: number; last: number };

const EMPTY_STATS: Stats = { count: 0, p50: 0, p95: 0, max: 0, last: 0 };

export function Bench() {
  const [cards, setCards] = useState<RepoCardData[]>(() => sampleCards(CARD_COUNT));
  const [query, setQuery] = useState("");
  const [stats, setStats] = useState<Stats>(EMPTY_STATS);

  const commits = useRef<number[]>([]);
  /**
   * Only commits caused by an interaction are recorded.
   *
   * Without this gate the bench measures itself: reporting the stats causes a
   * commit, which the profiler records, which changes the count, which reports
   * again — a render loop that also poisons the numbers.
   */
  const measuring = useRef(false);

  const onRender = useCallback<ProfilerOnRenderCallback>(
    (_id, _phase, actualDuration) => {
      if (!measuring.current) return;

      // Written to a ref rather than state: recording a commit in state would
      // cause another commit.
      commits.current.push(actualDuration);
    },
    [],
  );

  const handleRefresh = useCallback(() => undefined, []);
  const handleToggle = useCallback(() => undefined, []);
  const handleOpen = useCallback(() => undefined, []);

  /** Runs an interaction and reports the commits it produced. */
  const measure = useCallback((action: () => void) => {
    // Clear first so the readiness marker genuinely disappears and returns;
    // otherwise a reader cannot tell a fresh measurement from a stale one.
    setStats(EMPTY_STATS);
    commits.current = [];
    measuring.current = true;
    action();

    window.setTimeout(() => {
      measuring.current = false;
      setStats(summarise(commits.current));
    }, 400);
  }, []);

  function reset(): void {
    commits.current = [];
    setStats(EMPTY_STATS);
  }

  /** A single-card update: what a `run://updated` event actually costs. */
  function updateOne(): void {
    measure(() => {
      setCards((current) => {
        const target = current[0];
        if (!target) return current;

        return applyCardUpdate(current, {
          ...target,
          hasActiveRun: !target.hasActiveRun,
          failingRuns: target.failingRuns + 1,
        });
      });
    });
  }

  /** Fifty cards at once: a burst of webhook deliveries. */
  function updateFifty(): void {
    measure(() => {
      setCards((current) =>
        current.map((card, index) =>
          index % 5 === 0
            ? {
                ...card,
                hasActiveRun: !card.hasActiveRun,
                failingRuns: card.failingRuns + 1,
              }
            : card,
        ),
      );
    });
  }

  /** Every card at once: the cost of a full list refetch. */
  function replaceAll(): void {
    measure(() => {
      setCards(sampleCards(CARD_COUNT));
    });
  }

  /** Filtering, which is what the search box does on every keystroke. */
  function applyFilter(): void {
    measure(() => {
      setQuery((current) => (current === "service-1" ? "service-15" : "service-1"));
    });
  }

  const visible = useMemo(() => filterCards(cards, "all", query), [cards, query]);

  const grid = useMemo(
    () => (
      <DashboardGrid
        cards={visible}
        onRefresh={handleRefresh}
        onToggleMonitored={handleToggle}
        onOpen={handleOpen}
      />
    ),
    [visible, handleRefresh, handleToggle, handleOpen],
  );

  return (
    <div className="flex h-full min-h-0 flex-col">
      <Card className="m-4 shrink-0">
        <CardBody className="space-y-3">
          <div className="flex items-center gap-2">
            <h1 className="text-sm font-semibold">Render budget</h1>
            <span className="text-xs text-ink-muted">
              {visible.length} of {cards.length} repositories · budget: p95 commit &lt; 16
              ms
            </span>
            <Button
              size="sm"
              variant="ghost"
              className="ml-auto"
              onClick={() => {
                navigate("/");
              }}
            >
              Back
            </Button>
          </div>

          <div className="flex flex-wrap gap-4 font-mono text-xs">
            <Metric label="commits" value={String(stats.count)} />
            <Metric
              label="p50"
              value={`${stats.p50.toFixed(1)} ms`}
              tone={stats.p50 > 16 ? "bad" : "ok"}
            />
            <Metric
              label="p95"
              value={`${stats.p95.toFixed(1)} ms`}
              tone={stats.p95 > 16 ? "bad" : "ok"}
            />
            <Metric
              label="max"
              value={`${stats.max.toFixed(1)} ms`}
              tone={stats.max > 50 ? "bad" : "ok"}
            />
            <Metric label="last" value={`${stats.last.toFixed(1)} ms`} />
          </div>

          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" onClick={updateOne}>
              Update 1 card
            </Button>
            <Button size="sm" variant="secondary" onClick={updateFifty}>
              Update 50 cards
            </Button>
            <Button size="sm" variant="secondary" onClick={replaceAll}>
              Replace all
            </Button>
            <Button size="sm" variant="secondary" onClick={applyFilter}>
              Filter
            </Button>
            <Button size="sm" variant="ghost" onClick={reset}>
              Reset counters
            </Button>
          </div>

          {/*
            Machine-readable readiness marker: the measurement finishes on a
            timer, so anything reading this page needs to know when the numbers
            are real rather than still zero.
          */}
          {stats.count > 0 ? (
            <span className="sr-only" role="status">
              measurement-ready
            </span>
          ) : null}
        </CardBody>
      </Card>

      <Profiler id="dashboard-grid" onRender={onRender}>
        {grid}
      </Profiler>
    </div>
  );
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "ok" | "bad";
}) {
  const colour = tone === "bad" ? "text-bad" : tone === "ok" ? "text-ok" : "text-ink";

  return (
    <div className="flex items-baseline gap-1.5">
      <span className="text-ink-muted">{label}</span>
      <span className={`tabular-nums ${colour}`}>{value}</span>
    </div>
  );
}

function summarise(durations: number[]): Stats {
  if (durations.length === 0) return EMPTY_STATS;

  const sorted = [...durations].sort((left, right) => left - right);
  const at = (quantile: number) =>
    sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * quantile))] ?? 0;

  return {
    count: sorted.length,
    p50: at(0.5),
    p95: at(0.95),
    max: sorted[sorted.length - 1] ?? 0,
    last: durations[durations.length - 1] ?? 0,
  };
}

/** Deterministic sample data, so runs are comparable. */
function sampleCards(count: number): RepoCardData[] {
  const now = Date.now();

  return Array.from({ length: count }, (_, index) => {
    const statuses = ["completed", "in_progress", "queued"] as const;
    const conclusions = ["success", "failure", "cancelled", null] as const;

    const status = statuses[index % statuses.length] ?? "completed";
    const conclusion = status === "completed" ? (conclusions[index % 4] ?? null) : null;

    return {
      repo: {
        id: index + 1,
        githubId: 1000 + index,
        owner: "acme",
        name: `service-${index}`,
        fullName: `acme/service-${index}`,
        isPrivate: index % 3 === 0,
        defaultBranch: "main",
        avatarUrl: null,
        monitored: true,
        lastPolledAt: new Date(now - index * 1000).toISOString(),
        createdAt: new Date(now - 86_400_000).toISOString(),
        updatedAt: new Date(now).toISOString(),
      },
      recentRuns: Array.from({ length: 10 }, (_, runIndex) => ({
        id: index * 10 + runIndex + 1,
        repoId: index + 1,
        githubId: 500_000 + index * 10 + runIndex,
        workflowName: "ci",
        displayTitle: `Run ${runIndex}`,
        event: "push",
        branch: "main",
        headSha: "a1b2c3d4e5f6a7b8",
        status: runIndex === 0 ? status : "completed",
        conclusion: runIndex === 0 ? conclusion : "success",
        runNumber: 100 - runIndex,
        runAttempt: 1,
        htmlUrl: `https://github.com/acme/service-${index}/actions/runs/${runIndex}`,
        actorLogin: "octocat",
        createdAt: new Date(now - runIndex * 60_000).toISOString(),
        runStartedAt: new Date(now - runIndex * 60_000 - 5_000).toISOString(),
        updatedAt: new Date(now - runIndex * 60_000).toISOString(),
      })),
      failingRuns: index % 4 === 1 ? 3 : 0,
      hasActiveRun: status === "in_progress",
    };
  });
}
