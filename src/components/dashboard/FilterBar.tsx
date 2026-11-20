import { Search } from "lucide-react";
import { cn } from "@/lib/cn";
import type { RepoFilter } from "@/lib/repo-selectors";
import { Button } from "@/components/primitives/Button";

const FILTERS: Array<{ id: RepoFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "failing", label: "Failing" },
  { id: "running", label: "Running" },
];

export interface FilterBarProps {
  query: string;
  filter: RepoFilter;
  onQueryChange: (query: string) => void;
  onFilterChange: (filter: RepoFilter) => void;
  onRefreshAll: () => void;
  refreshing: boolean;
}

export function FilterBar({
  query,
  filter,
  onQueryChange,
  onFilterChange,
  onRefreshAll,
  refreshing,
}: FilterBarProps) {
  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-line px-4 py-2">
      <div className="relative min-w-48 flex-1">
        <Search
          className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-ink-muted"
          aria-hidden="true"
        />
        <input
          type="search"
          value={query}
          onChange={(event) => {
            onQueryChange(event.target.value);
          }}
          placeholder="Filter repositories…"
          aria-label="Filter repositories"
          className={cn(
            "h-8 w-full rounded-control border border-line bg-raised pl-8 pr-3 text-[13px] text-ink",
            "placeholder:text-ink-muted",
            "focus-visible:border-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40",
          )}
        />
      </div>

      <div role="group" aria-label="Filter repositories by state" className="flex gap-1">
        {FILTERS.map((entry) => (
          <Button
            key={entry.id}
            size="sm"
            variant={filter === entry.id ? "secondary" : "ghost"}
            aria-pressed={filter === entry.id}
            onClick={() => {
              onFilterChange(entry.id);
            }}
          >
            {entry.label}
          </Button>
        ))}
      </div>

      <Button size="sm" variant="secondary" loading={refreshing} onClick={onRefreshAll}>
        Refresh all
      </Button>
    </div>
  );
}
