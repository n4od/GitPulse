import { RefreshCw, Search } from "lucide-react";
import { cn } from "@/lib/cn";
import type { RepoFilter } from "@/lib/repo-selectors";
import { Button } from "@/components/primitives/Button";

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
  onQueryChange: (query: string) => void;
  onFilterChange: (filter: RepoFilter) => void;
  onRefreshAll: () => void;
  refreshing: boolean;
}

/**
 * The dashboard header: title, watch count, state filters, search, refresh.
 *
 * Everything a user reaches for while watching builds sits on one row, so the
 * grid below is never pushed down by a second toolbar.
 */
export function FilterBar({
  query,
  filter,
  monitoredCount,
  onQueryChange,
  onFilterChange,
  onRefreshAll,
  refreshing,
}: FilterBarProps) {
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-3 px-8 pb-4 pt-6">
      <div className="flex items-center gap-3">
        <h1 className="text-[22px] font-bold tracking-tight text-ink">Repositories</h1>
        <span className="rounded-full border border-line bg-raised px-2.5 py-0.5 text-[11px] text-ink-muted">
          {monitoredCount} monitored
        </span>
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
              "focus-visible:border-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
            )}
          />
        </div>

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
