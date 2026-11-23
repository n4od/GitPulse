import { LayoutGrid, Plus, RefreshCw, Rows4, Search, User } from "lucide-react";
import { cn } from "@/lib/cn";
import type { RepoFilter } from "@/lib/repo-selectors";
import type { Density } from "@/stores/ui";
import { Button } from "@/components/primitives/Button";
import { Tooltip } from "@/components/primitives/Tooltip";

const FILTERS: Array<{ id: RepoFilter; label: string; dot?: string }> = [
  { id: "all", label: "All" },
  { id: "failing", label: "Failing", dot: "bg-bad" },
  { id: "running", label: "Running", dot: "bg-live" },
];

export interface FilterBarProps {
  query: string;
  filter: RepoFilter;
  /** How many repositories are watched, shown next to the title. */
  monitoredCount: number;
  /** Show only repositories owned by the signed-in account. */
  mineOnly: boolean;
  /** False when nobody is signed in, because then "mine" means nothing. */
  canFilterMine: boolean;
  density: Density;
  onQueryChange: (query: string) => void;
  onFilterChange: (filter: RepoFilter) => void;
  onMineChange: (mineOnly: boolean) => void;
  onToggleDensity: () => void;
  onAddRepositories: () => void;
  onRefreshAll: () => void;
  refreshing: boolean;
}

/**
 * The dashboard header: title, watch count, state filters, ownership filter,
 * search, card size and refresh.
 *
 * Everything a user reaches for while watching builds sits on one row, so the
 * grid below is never pushed down by a second toolbar.
 */
export function FilterBar({
  query,
  filter,
  monitoredCount,
  mineOnly,
  canFilterMine,
  density,
  onQueryChange,
  onFilterChange,
  onMineChange,
  onToggleDensity,
  onAddRepositories,
  onRefreshAll,
  refreshing,
}: FilterBarProps) {
  const compact = density === "compact";

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-3 px-8 pb-4 pt-6">
      <div className="flex items-center gap-3">
        <h1 className="text-[22px] font-bold tracking-tight text-ink">Repositories</h1>
        <span className="rounded-full border border-line bg-raised px-2.5 py-0.5 text-[11px] text-ink-muted">
          {monitoredCount} monitored
        </span>
        <Button
          variant="secondary"
          size="sm"
          leadingIcon={<Plus className="size-3.5" />}
          onClick={onAddRepositories}
        >
          Add repositories
        </Button>
      </div>

      <div className="ml-auto flex flex-wrap items-center gap-2">
        <div
          role="group"
          aria-label="Filter repositories by state"
          className="flex gap-1 rounded-[10px] border border-line bg-raised p-1"
        >
          {FILTERS.map((entry) => {
            const active = filter === entry.id;

            return (
              <button
                key={entry.id}
                type="button"
                aria-pressed={active}
                onClick={() => {
                  onFilterChange(entry.id);
                }}
                className={cn(
                  "flex items-center gap-1.5 rounded-[7px] px-3 py-1.5 text-[12px] transition-colors",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
                  active
                    ? "bg-overlay font-semibold text-ink"
                    : "text-ink-soft hover:text-ink",
                )}
              >
                {entry.dot ? (
                  <span
                    className={cn("size-1.5 rounded-full", entry.dot)}
                    aria-hidden="true"
                  />
                ) : null}
                {entry.label}
              </button>
            );
          })}
        </div>

        {canFilterMine ? (
          <button
            type="button"
            aria-pressed={mineOnly}
            onClick={() => {
              onMineChange(!mineOnly);
            }}
            className={cn(
              "flex h-9 items-center gap-1.5 rounded-[10px] border px-3 text-[12px] transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
              mineOnly
                ? "border-accent/40 bg-accent/10 font-semibold text-ink"
                : "border-line bg-raised text-ink-soft hover:text-ink",
            )}
          >
            <User className="size-3.5" aria-hidden="true" />
            Mine
          </button>
        ) : null}

        <div className="relative w-52">
          <Search
            className="pointer-events-none absolute left-3 top-1/2 size-3.5 -translate-y-1/2 text-ink-muted"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(event) => {
              onQueryChange(event.target.value);
            }}
            placeholder="Search repositories"
            aria-label="Search repositories"
            className={cn(
              "h-9 w-full rounded-control border border-line bg-raised pl-9 pr-3 text-[12px] text-ink",
              "placeholder:text-ink-muted",
            )}
          />
        </div>

        <Tooltip label={compact ? "Comfortable cards" : "Compact cards"}>
          <button
            type="button"
            aria-label={
              compact ? "Switch to comfortable cards" : "Switch to compact cards"
            }
            aria-pressed={compact}
            onClick={onToggleDensity}
            className={cn(
              "flex size-9 items-center justify-center rounded-[10px] border border-line bg-raised",
              "text-ink-soft transition-colors hover:text-ink",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
            )}
          >
            {compact ? <Rows4 className="size-4" /> : <LayoutGrid className="size-4" />}
          </button>
        </Tooltip>

        <Button
          variant="primary"
          size="md"
          loading={refreshing}
          leadingIcon={<RefreshCw className="size-3.5" />}
          className="bg-ink text-surface hover:bg-ink/90 active:bg-ink/80"
          onClick={onRefreshAll}
        >
          Refresh all
        </Button>
      </div>
    </div>
  );
}
