/**
 * Command palette matching.
 *
 * A subsequence match, not a substring one: "acapi" should find "acme/api",
 * because the point of a palette is to type three or four characters and hit
 * enter. Scoring prefers earlier matches and word boundaries, so "api" puts
 * `acme/api` above `acme/api-service`.
 */

export interface PaletteEntry {
  id: string;
  label: string;
  detail?: string;
}

/** Lower is better. `null` means "does not match". */
export function score(label: string, query: string): number | null {
  const needle = query.trim().toLowerCase();
  if (needle === "") return 0;

  const haystack = label.toLowerCase();
  let position = 0;
  let total = 0;
  let previousIndex = -1;

  for (const character of needle) {
    const index = haystack.indexOf(character, position);
    if (index === -1) return null;

    // Contiguous matches and word starts are worth more than scattered letters.
    const isBoundary = index === 0 || "/-_ .".includes(haystack[index - 1] ?? "");
    total += index - previousIndex;
    if (isBoundary) total -= 2;

    previousIndex = index;
    position = index + 1;
  }

  return total;
}

export function filterEntries<T extends PaletteEntry>(
  entries: readonly T[],
  query: string,
  limit: number,
): T[] {
  const scored: Array<{ entry: T; value: number }> = [];

  for (const entry of entries) {
    const value = score(entry.label, query);
    if (value !== null) scored.push({ entry, value });
  }

  scored.sort(
    (left, right) =>
      left.value - right.value || left.entry.label.localeCompare(right.entry.label),
  );

  return scored.slice(0, limit).map((item) => item.entry);
}

/** The static half of the palette. */
export const PALETTE_ACTIONS: PaletteEntry[] = [
  { id: "track-repos", label: "Track repositories", detail: "Action" },
  { id: "settings", label: "Open settings", detail: "Settings" },
  { id: "refresh-all", label: "Refresh every repository", detail: "Action" },
  { id: "check-updates", label: "Check for updates", detail: "Action" },
  { id: "sign-out", label: "Sign out of GitHub", detail: "Account" },
];
