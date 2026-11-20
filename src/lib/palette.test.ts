import { describe, expect, it } from "vitest";
import { filterEntries, score, type PaletteEntry } from "@/lib/palette";

const ENTRIES: PaletteEntry[] = [
  { id: "a", label: "acme/api" },
  { id: "b", label: "acme/api-service" },
  { id: "c", label: "acme/website" },
  { id: "d", label: "other/service" },
];

describe("score", () => {
  it("matches everything on an empty query", () => {
    expect(score("anything", "")).toBe(0);
  });

  it("rejects a query whose letters are not all present in order", () => {
    expect(score("acme/api", "apex")).toBeNull();
    expect(score("acme/api", "zzz")).toBeNull();
  });

  it("accepts a subsequence", () => {
    expect(score("acme/api", "acapi")).not.toBeNull();
  });

  it("prefers a prefix over a later match", () => {
    const prefix = score("api-server", "api");
    const later = score("my-api-server", "api");

    expect(prefix).not.toBeNull();
    expect(later).not.toBeNull();
    expect(prefix ?? 0).toBeLessThan(later ?? 0);
  });
});

describe("filterEntries", () => {
  it("returns everything, capped, for an empty query", () => {
    expect(filterEntries(ENTRIES, "", 2)).toHaveLength(2);
  });

  it("puts the better match first", () => {
    const results = filterEntries(ENTRIES, "acme/api", 8);

    expect(results[0]?.id).toBe("a");
    expect(results[1]?.id).toBe("b");
  });

  it("does not return entries that do not match", () => {
    const results = filterEntries(ENTRIES, "website", 8);

    expect(results.map((entry) => entry.id)).toEqual(["c"]);
  });

  it("honours the limit", () => {
    expect(filterEntries(ENTRIES, "service", 1)).toHaveLength(1);
  });

  it("returns an empty list rather than throwing when nothing matches", () => {
    expect(filterEntries(ENTRIES, "qqqq", 8)).toEqual([]);
  });
});
