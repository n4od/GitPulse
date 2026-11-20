import type { RepoCard } from "@/lib/bindings/RepoCard";

/**
 * Shows a run's new status immediately, before GitHub confirms it.
 *
 * Used for re-run and cancel: the request usually succeeds, and waiting for a
 * round trip to acknowledge a button press feels broken. The caller keeps the
 * previous card and restores it if the request fails.
 */
export function optimisticallySetRunStatus(
  card: RepoCard,
  runId: number,
  status: string,
): RepoCard {
  return {
    ...card,
    recentRuns: card.recentRuns.map((run) =>
      run.id === runId ? { ...run, status, conclusion: null } : run,
    ),
  };
}

/** Splits a capped log body into lines, for a virtualised viewer. */
export function toLines(text: string): string[] {
  if (text === "") return [];
  // A trailing newline should not produce a phantom blank last line.
  return text.replace(/\n$/, "").split("\n");
}

const FAILURE_MARKERS = ["##[error]", "Error:", "error:", "FAILED", "failed"];

/**
 * Index of the first line that looks like a failure, or -1.
 *
 * Workflow logs mark failures with `##[error]`; the rest is best effort, and
 * being wrong just means the button lands somewhere unhelpful.
 */
export function firstFailureLine(lines: readonly string[]): number {
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (line === undefined) continue;
    if (FAILURE_MARKERS.some((marker) => line.includes(marker))) return index;
  }

  return -1;
}

/** Case-insensitive line search, returning the matching indices. */
export function matchingLines(lines: readonly string[], query: string): number[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return [];

  return lines.reduce<number[]>((matches, line, index) => {
    if (line.toLowerCase().includes(needle)) matches.push(index);
    return matches;
  }, []);
}
