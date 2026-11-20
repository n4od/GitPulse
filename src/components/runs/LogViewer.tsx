import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, Copy, Search } from "lucide-react";
import type { LogDownload } from "@/lib/bindings/LogDownload";
import { Button } from "@/components/primitives/Button";
import { cn } from "@/lib/cn";
import { firstFailureLine, matchingLines, toLines } from "@/lib/run-selectors";
import { toast } from "@/stores/toasts";

/** Matches the backend's fetch cap so the UI cannot ask for more than it has. */
const LINE_RENDER_LIMIT = 20_000;
const LINE_HEIGHT = 18;

export function LogViewer({ download }: { download: LogDownload }) {
  const [query, setQuery] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  const lines = useMemo(() => {
    const all = toLines(download.text);
    // The backend caps bytes, not lines; a pathological log could still be
    // enormous, and a viewer that freezes is worse than one that truncates.
    return all.length > LINE_RENDER_LIMIT ? all.slice(0, LINE_RENDER_LIMIT) : all;
  }, [download.text]);

  const matches = useMemo(() => matchingLines(lines, query), [lines, query]);
  const failureIndex = useMemo(() => firstFailureLine(lines), [lines]);
  const linesTruncated = lines.length >= LINE_RENDER_LIMIT;

  // See DashboardGrid: TanStack Virtual is deliberately stateful.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: lines.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => LINE_HEIGHT,
    overscan: 20,
  });

  // Scrolling a virtualised list means asking the virtualizer, not the DOM.
  useEffect(() => {
    if (failureIndex >= 0) virtualizer.scrollToIndex(failureIndex, { align: "center" });
    // Deliberately only on a new log: re-scrolling on every render would fight
    // the user's own scrolling.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [download.text, failureIndex]);

  function jumpTo(index: number): void {
    if (index >= 0) virtualizer.scrollToIndex(index, { align: "center" });
  }

  async function copyAll(): Promise<void> {
    try {
      await navigator.clipboard.writeText(download.text);
      toast("success", "Log copied");
    } catch {
      toast("error", "Could not copy the log");
    }
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b border-line px-3 py-2">
        <div className="relative min-w-40 flex-1">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-ink-muted"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") jumpTo(matches[0] ?? -1);
            }}
            placeholder="Search the log…"
            aria-label="Search the log"
            className="h-7 w-full rounded-control border border-line bg-raised pl-8 pr-2 text-xs text-ink placeholder:text-ink-muted focus-visible:border-accent focus-visible:outline-none"
          />
        </div>

        <span className="font-mono text-[11px] tabular-nums text-ink-muted">
          {matches.length} match{matches.length === 1 ? "" : "es"}
        </span>

        <Button
          size="sm"
          variant="ghost"
          aria-label="Jump to the first failure"
          disabled={failureIndex < 0}
          onClick={() => {
            jumpTo(failureIndex);
          }}
        >
          <ChevronDown className="size-3.5" />
          Failure
        </Button>

        <Button
          size="sm"
          variant="ghost"
          aria-label="Copy the whole log"
          onClick={() => {
            void copyAll();
          }}
        >
          <Copy className="size-3.5" />
        </Button>
      </div>

      {download.truncated || linesTruncated ? (
        <div className="shrink-0 border-b border-warn/30 bg-warn/10 px-3 py-1 text-[11px] text-warn">
          This log was cut short — Pulse caps a single download at 5 MB.
        </div>
      ) : null}

      <div ref={scrollRef} className="selectable min-h-0 flex-1 overflow-auto bg-surface">
        <div className="relative" style={{ height: virtualizer.getTotalSize() }}>
          {virtualizer.getVirtualItems().map((row) => {
            const line = lines[row.index] ?? "";
            const isMatch = matches.includes(row.index);
            const isFailure = row.index === failureIndex;

            return (
              <div
                key={row.key}
                className={cn(
                  "absolute left-0 top-0 flex w-full gap-3 px-3 font-mono text-[11px] leading-[18px]",
                  isFailure && "bg-bad/15",
                  !isFailure && isMatch && "bg-accent/10",
                )}
                style={{ height: LINE_HEIGHT, transform: `translateY(${row.start}px)` }}
              >
                <span className="w-10 shrink-0 select-none text-right text-ink-muted">
                  {row.index + 1}
                </span>
                <span className="whitespace-pre text-ink-soft">{line}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
