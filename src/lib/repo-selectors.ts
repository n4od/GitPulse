import type { RepoCard } from "@/lib/bindings/RepoCard";
import type { WorkflowRun } from "@/lib/bindings/WorkflowRun";

export type RepoFilter = "all" | "failing" | "running";

/** The newest run, or null when the repository has never run. */
export function latestRun(card: RepoCard): WorkflowRun | null {
  return card.recentRuns[0] ?? null;
}

export function filterCards(
  cards: readonly RepoCard[],
  filter: RepoFilter,
  query: string,
): RepoCard[] {
  const needle = query.trim().toLowerCase();

  return cards.filter((card) => {
    if (filter === "failing" && card.failingRuns === 0) return false;
    if (filter === "running" && !card.hasActiveRun) return false;
    if (needle !== "" && !card.repo.fullName.toLowerCase().includes(needle)) return false;
    return true;
  });
}

/**
 * Attention first: repositories with something in flight, then ones with
 * failures, then alphabetically so the order is stable between renders.
 */
export function sortCards(cards: readonly RepoCard[]): RepoCard[] {
  return [...cards].sort((left, right) => {
    if (left.hasActiveRun !== right.hasActiveRun) return left.hasActiveRun ? -1 : 1;

    const leftFailing = left.failingRuns > 0 ? 0 : 1;
    const rightFailing = right.failingRuns > 0 ? 0 : 1;
    if (leftFailing !== rightFailing) return leftFailing - rightFailing;

    return left.repo.fullName.localeCompare(right.repo.fullName);
  });
}

/**
 * Replaces exactly one card.
 *
 * This is the surgical update the live bridge performs on `run://updated`: the
 * backend sends the whole recomputed card, so the UI never refetches the list
 * and the card cannot drift from the database.
 */
export function applyCardUpdate(cards: readonly RepoCard[], card: RepoCard): RepoCard[] {
  const index = cards.findIndex((existing) => existing.repo.id === card.repo.id);

  if (index === -1) {
    // A repository we did not know about: a newly tracked one.
    return [...cards, card];
  }

  const next = cards.slice();
  next[index] = card;
  return next;
}

/** Compact relative time, e.g. "3m ago". */
export function relativeTime(iso: string | null, now: number = Date.now()): string {
  if (!iso) return "never";

  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "unknown";

  const seconds = Math.max(0, Math.round((now - then) / 1000));
  if (seconds < 60) return `${seconds}s ago`;

  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  return `${Math.round(hours / 24)}d ago`;
}

/** How long a run took, or "running" while it is still going. */
export function runDuration(run: WorkflowRun, now: number = Date.now()): string {
  const started = run.runStartedAt ? Date.parse(run.runStartedAt) : NaN;
  if (Number.isNaN(started)) return "";

  const finished = run.conclusion ? Date.parse(run.updatedAt) : now;
  const seconds = Math.max(0, Math.round((finished - started) / 1000));

  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return remainder === 0 ? `${minutes}m` : `${minutes}m ${remainder}s`;
}

export function shortSha(sha: string | null): string {
  return sha ? sha.slice(0, 7) : "";
}
