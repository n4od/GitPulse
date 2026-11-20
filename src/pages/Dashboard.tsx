import { useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Activity, Plus } from "lucide-react";
import { DashboardGrid } from "@/components/dashboard/DashboardGrid";
import { FilterBar } from "@/components/dashboard/FilterBar";
import { RepoCardSkeletonGrid } from "@/components/dashboard/RepoCardSkeleton";
import { RepoPicker } from "@/components/dashboard/RepoPicker";
import { EmptyState } from "@/components/shell/EmptyState";
import { Button } from "@/components/primitives/Button";
import { REPOS_QUERY_KEY, useRepos } from "@/hooks/useRepos";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { filterCards, sortCards, type RepoFilter } from "@/lib/repo-selectors";
import { toast } from "@/stores/toasts";

/**
 * The dashboard.
 *
 * Data arrives once, then updates arrive as `run://updated` events that patch
 * individual cards. Nothing here invalidates the list on a run change — that
 * would refetch all 250 cards because one build finished.
 */
export function Dashboard() {
  const queryClient = useQueryClient();
  const repos = useRepos(true);

  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<RepoFilter>("all");
  const [picking, setPicking] = useState(false);

  const cards = useMemo(
    () => sortCards(filterCards(repos.data ?? [], filter, query)),
    [repos.data, filter, query],
  );

  function queueRefresh(repoId: number): void {
    void call("repos_refresh", { repoId }).catch((cause: unknown) => {
      toast("error", "Could not queue a refresh", toPulseError(cause).message);
    });
  }

  async function toggleMonitored(repoId: number, monitored: boolean): Promise<void> {
    try {
      await call("repos_set_monitored", { repoId, monitored });
      // A mute is structural: the card's own fields changed, and no poll will
      // announce it.
      await queryClient.invalidateQueries({ queryKey: REPOS_QUERY_KEY });
    } catch (cause) {
      toast("error", "Could not update that repository", toPulseError(cause).message);
    }
  }

  async function refreshAll(): Promise<void> {
    const ids = (repos.data ?? []).map((card) => card.repo.id);

    await Promise.all(ids.map((repoId) => call("repos_refresh", { repoId })));
    toast("info", `Queued ${ids.length} ${ids.length === 1 ? "refresh" : "refreshes"}`);
  }

  if (picking) {
    return (
      <div className="overflow-y-auto p-4">
        <RepoPicker
          onClose={() => {
            setPicking(false);
          }}
        />
      </div>
    );
  }

  if (repos.isPending) {
    return (
      <div className="p-4">
        <RepoCardSkeletonGrid />
      </div>
    );
  }

  if (repos.isError) {
    return (
      <EmptyState
        title="Could not load your repositories"
        description={toPulseError(repos.error).message}
        action={
          <Button
            size="sm"
            variant="primary"
            onClick={() => {
              void repos.refetch();
            }}
          >
            Try again
          </Button>
        }
      />
    );
  }

  if ((repos.data ?? []).length === 0) {
    return (
      <EmptyState
        icon={<Activity className="size-5" aria-hidden="true" />}
        title="No repositories yet"
        description="Choose the repositories Pulse should watch. Nothing is polled until you pick one."
        action={
          <Button
            size="sm"
            variant="primary"
            leadingIcon={<Plus className="size-3.5" />}
            onClick={() => {
              setPicking(true);
            }}
          >
            Choose repositories
          </Button>
        }
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <FilterBar
        query={query}
        filter={filter}
        onQueryChange={setQuery}
        onFilterChange={setFilter}
        onRefreshAll={() => {
          void refreshAll();
        }}
        refreshing={repos.isFetching}
      />

      {cards.length === 0 ? (
        <EmptyState
          title="Nothing matches"
          description="No repository matches that filter and search term."
        />
      ) : (
        <DashboardGrid
          cards={cards}
          onRefresh={queueRefresh}
          onToggleMonitored={(repoId, monitored) => {
            void toggleMonitored(repoId, monitored);
          }}
        />
      )}
    </div>
  );
}
