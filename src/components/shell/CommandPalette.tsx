import { useEffect, useMemo, useRef, useState } from "react";
import { Search } from "lucide-react";
import type { RepoCard } from "@/lib/bindings/RepoCard";
import { filterEntries, PALETTE_ACTIONS, type PaletteEntry } from "@/lib/palette";
import { cn } from "@/lib/cn";

const MAX_RESULTS = 8;

interface CommandPaletteProps {
  cards: readonly RepoCard[];
  /** Accounts already signed in, so switching is one keystroke away. */
  accounts?: readonly string[];
  onClose: () => void;
  onOpenRepo: (repoId: number) => void;
  onSwitchAccount?: (login: string) => void;
  onAction: (id: string) => void;
}

/**
 * `Ctrl`/`Cmd`+`K`: type a few characters, land on a repository or an action.
 *
 * Keyboard-first by construction — the input is always focused, and the list is
 * navigated with the arrow keys rather than the mouse.
 */
export function CommandPalette({
  cards,
  accounts = [],
  onClose,
  onOpenRepo,
  onSwitchAccount,
  onAction,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const entries = useMemo<
    Array<PaletteEntry & { repoId?: number; accountLogin?: string }>
  >(
    () => [
      ...PALETTE_ACTIONS,
      ...accounts.map((login) => ({
        id: `account-${login}`,
        label: login,
        detail: "account",
        accountLogin: login,
      })),
      ...cards.map((card) => ({
        id: `repo-${card.repo.id}`,
        label: card.repo.fullName,
        detail: card.hasActiveRun ? "running" : card.failingRuns > 0 ? "failing" : "idle",
        repoId: card.repo.id,
      })),
    ],
    [accounts, cards],
  );

  const results = useMemo(
    () => filterEntries(entries, query, MAX_RESULTS),
    [entries, query],
  );

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Keep the highlighted row inside the scroll viewport.
  useEffect(() => {
    const item = listRef.current?.children[active];

    // jsdom has no layout, and older WebViews lack this; a palette that throws
    // because it cannot scroll is worse than one that does not scroll.
    if (item && typeof item.scrollIntoView === "function") {
      item.scrollIntoView({ block: "nearest" });
    }
  }, [active]);

  function choose(index: number): void {
    const entry = results[index];
    if (!entry) return;

    if (entry.repoId !== undefined) {
      onOpenRepo(entry.repoId);
    } else if (entry.accountLogin !== undefined) {
      onSwitchAccount?.(entry.accountLogin);
    } else {
      onAction(entry.id);
    }

    onClose();
  }

  return (
    <div
      className="absolute inset-0 z-50 flex items-start justify-center bg-surface/70 px-4 pt-24"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-card border border-line-strong bg-overlay shadow-2xl"
        onClick={(event) => {
          event.stopPropagation();
        }}
        role="dialog"
        aria-modal="true"
        aria-label="Command palette"
      >
        <div className="relative border-b border-line">
          <Search
            className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-ink-muted"
            aria-hidden="true"
          />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActive(0);
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowDown") {
                event.preventDefault();
                setActive((current) => Math.min(current + 1, results.length - 1));
              } else if (event.key === "ArrowUp") {
                event.preventDefault();
                setActive((current) => Math.max(current - 1, 0));
              } else if (event.key === "Enter") {
                event.preventDefault();
                choose(active);
              }
            }}
            placeholder="Jump to a repository or run an action…"
            aria-label="Search repositories and actions"
            className="h-11 w-full bg-transparent pl-10 pr-3 text-sm text-ink placeholder:text-ink-muted focus-visible:outline-none"
          />
        </div>

        <ul ref={listRef} role="listbox" className="max-h-80 overflow-y-auto py-1">
          {results.length === 0 ? (
            <li className="px-4 py-6 text-center text-xs text-ink-muted">
              Nothing matches.
            </li>
          ) : (
            results.map((entry, index) => (
              <li key={entry.id} role="option" aria-selected={index === active}>
                <button
                  type="button"
                  onMouseEnter={() => {
                    setActive(index);
                  }}
                  onClick={() => {
                    choose(index);
                  }}
                  className={cn(
                    "flex w-full items-center gap-2 px-4 py-2 text-left text-[13px]",
                    index === active ? "bg-raised text-ink" : "text-ink-soft",
                  )}
                >
                  <span className="truncate">{entry.label}</span>
                  {entry.detail ? (
                    <span className="ml-auto shrink-0 text-[11px] text-ink-muted">
                      {entry.detail}
                    </span>
                  ) : null}
                </button>
              </li>
            ))
          )}
        </ul>

        <div className="flex items-center gap-3 border-t border-line px-4 py-1.5 text-[10px] text-ink-muted">
          <span>↑↓ move</span>
          <span>↵ open</span>
          <span>esc close</span>
        </div>
      </div>
    </div>
  );
}
