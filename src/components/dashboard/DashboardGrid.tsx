import { useEffect, useRef, useState, type RefObject } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { RepoCard as RepoCardData } from "@/lib/bindings/RepoCard";
import { RepoCard } from "@/components/dashboard/RepoCard";

/** Matches the CSS grid's `minmax(320px, 1fr)` so the columns line up. */
const COLUMN_MIN_WIDTH = 320;
const ROW_GAP = 12;
const ROW_HEIGHT = 160;

export interface DashboardGridProps {
  cards: RepoCardData[];
  onRefresh: (repoId: number) => void;
  onToggleMonitored: (repoId: number, monitored: boolean) => void;
  onOpen: (repoId: number) => void;
}

export function DashboardGrid({
  cards,
  onRefresh,
  onToggleMonitored,
  onOpen,
}: DashboardGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const columns = useColumnCount(scrollRef);
  const rows = Math.ceil(cards.length / columns);

  // One rendering path for every list size. An earlier version switched to a
  // plain grid below 120 cards, and *crossing* that boundary cost 272 ms in the
  // profiler: a filter that dropped from 250 to 111 cards tore down every row
  // and rebuilt them in a different mode. Virtualising always removes the
  // cliff, and costs nothing measurable for short lists.
  // TanStack Virtual's virtualizer is deliberately stateful and cannot be
  // memoised by the React compiler; it is only used for row offsets here.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: Math.max(rows, 1),
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT + ROW_GAP,
    overscan: 4,
  });

  return (
    <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto p-4">
      <div className="relative" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((row) => {
          const start = row.index * columns;

          return (
            <div
              key={row.key}
              className="absolute left-0 top-0 grid w-full gap-3"
              style={{
                gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                height: ROW_HEIGHT,
                transform: `translateY(${row.start}px)`,
              }}
            >
              {cards.slice(start, start + columns).map((card) => (
                <RepoCard
                  key={card.repo.id}
                  card={card}
                  onRefresh={onRefresh}
                  onToggleMonitored={onToggleMonitored}
                  onOpen={onOpen}
                />
              ))}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Derives the column count from the container's measured width.
 *
 * `ResizeObserver` fires once on observe, so there is no synchronous
 * `setState` in the effect — the initial value is corrected by the observer.
 * Where there is no observer (jsdom), the default stands.
 */
function useColumnCount(ref: RefObject<HTMLDivElement | null>): number {
  const [columns, setColumns] = useState(3);

  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === "undefined") return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;

      const fitted = Math.floor(entry.contentRect.width / COLUMN_MIN_WIDTH);
      setColumns(Math.max(1, fitted));
    });

    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [ref]);

  return columns;
}
