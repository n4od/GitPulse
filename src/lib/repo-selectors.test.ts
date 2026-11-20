import { describe, expect, it } from "vitest";
import { makeCard, makeRepo, makeRun } from "@/test/fixtures";
import {
  applyCardUpdate,
  filterCards,
  latestRun,
  relativeTime,
  runDuration,
  shortSha,
  sortCards,
} from "@/lib/repo-selectors";

describe("filterCards", () => {
  const failing = makeCard({
    repo: makeRepo({ id: 2, fullName: "acme/web" }),
    failingRuns: 2,
  });
  const running = makeCard({
    repo: makeRepo({ id: 3, fullName: "acme/worker" }),
    hasActiveRun: true,
  });

  it("passes everything through on the all filter", () => {
    expect(filterCards([failing, running], "all", "")).toHaveLength(2);
  });

  it("keeps only repositories with failures", () => {
    expect(filterCards([failing, running], "failing", "").map((c) => c.repo.id)).toEqual([
      2,
    ]);
  });

  it("keeps only repositories with something in flight", () => {
    expect(filterCards([failing, running], "running", "").map((c) => c.repo.id)).toEqual([
      3,
    ]);
  });

  it("matches the search term case-insensitively", () => {
    expect(filterCards([failing, running], "all", "WEB").map((c) => c.repo.id)).toEqual([
      2,
    ]);
  });

  it("ignores surrounding whitespace in the search term", () => {
    expect(filterCards([failing, running], "all", "  worker  ")).toHaveLength(1);
  });

  it("combines the filter and the search term", () => {
    expect(filterCards([failing, running], "failing", "worker")).toHaveLength(0);
  });
});

describe("sortCards", () => {
  it("puts running repositories first, then failing, then alphabetical", () => {
    const cards = [
      makeCard({ repo: makeRepo({ id: 1, fullName: "acme/zeta" }) }),
      makeCard({ repo: makeRepo({ id: 2, fullName: "acme/beta" }), failingRuns: 3 }),
      makeCard({ repo: makeRepo({ id: 3, fullName: "acme/alpha" }), hasActiveRun: true }),
      makeCard({ repo: makeRepo({ id: 4, fullName: "acme/gamma" }), failingRuns: 1 }),
    ];

    expect(sortCards(cards).map((card) => card.repo.fullName)).toEqual([
      "acme/alpha",
      "acme/beta",
      "acme/gamma",
      "acme/zeta",
    ]);
  });

  it("does not mutate its input", () => {
    const cards = [
      makeCard({ repo: makeRepo({ id: 1, fullName: "acme/zeta" }) }),
      makeCard({ repo: makeRepo({ id: 2, fullName: "acme/alpha" }) }),
    ];
    const before = cards.map((card) => card.repo.fullName);

    sortCards(cards);

    expect(cards.map((card) => card.repo.fullName)).toEqual(before);
  });
});

describe("applyCardUpdate", () => {
  it("replaces the card with the same repository id", () => {
    const cards = [makeCard({ repo: makeRepo({ id: 7 }), failingRuns: 0 })];
    const updated = makeCard({ repo: makeRepo({ id: 7 }), failingRuns: 4 });

    const next = applyCardUpdate(cards, updated);

    expect(next).toHaveLength(1);
    expect(next[0]?.failingRuns).toBe(4);
  });

  it("appends a repository the client has not seen", () => {
    const cards = [makeCard({ repo: makeRepo({ id: 7 }) })];
    const fresh = makeCard({ repo: makeRepo({ id: 8, fullName: "acme/new" }) });

    const next = applyCardUpdate(cards, fresh);

    expect(next.map((card) => card.repo.id)).toEqual([7, 8]);
  });

  it("leaves the other cards untouched, by reference", () => {
    const untouched = makeCard({ repo: makeRepo({ id: 1, fullName: "acme/one" }) });
    const cards = [
      untouched,
      makeCard({ repo: makeRepo({ id: 2, fullName: "acme/two" }) }),
    ];

    const next = applyCardUpdate(
      cards,
      makeCard({ repo: makeRepo({ id: 2, fullName: "acme/two" }), failingRuns: 9 }),
    );

    expect(next[0]).toBe(untouched);
  });

  it("does not mutate its input", () => {
    const cards = [makeCard({ repo: makeRepo({ id: 7 }) })];
    applyCardUpdate(cards, makeCard({ repo: makeRepo({ id: 7 }), failingRuns: 3 }));

    expect(cards[0]?.failingRuns).toBe(0);
  });
});

describe("latestRun", () => {
  it("returns the newest run", () => {
    const card = makeCard({
      recentRuns: [makeRun({ githubId: 2 }), makeRun({ githubId: 1 })],
    });
    expect(latestRun(card)?.githubId).toBe(2);
  });

  it("returns null when there is no history", () => {
    expect(latestRun(makeCard({ recentRuns: [] }))).toBeNull();
  });
});

describe("relativeTime", () => {
  const now = Date.parse("2026-11-20T12:00:00Z");

  it("renders seconds, minutes, hours and days", () => {
    expect(relativeTime("2026-11-20T11:59:30Z", now)).toBe("30s ago");
    expect(relativeTime("2026-11-20T11:45:00Z", now)).toBe("15m ago");
    expect(relativeTime("2026-11-20T09:00:00Z", now)).toBe("3h ago");
    expect(relativeTime("2026-11-18T12:00:00Z", now)).toBe("2d ago");
  });

  it("handles a missing timestamp", () => {
    expect(relativeTime(null, now)).toBe("never");
  });

  it("handles an unparseable timestamp", () => {
    expect(relativeTime("not a date", now)).toBe("unknown");
  });

  it("never reports negative time for a clock skewed into the future", () => {
    expect(relativeTime("2026-11-20T12:00:30Z", now)).toBe("0s ago");
  });
});

describe("runDuration", () => {
  const now = Date.parse("2026-11-20T12:00:00Z");

  it("reports a finished run in minutes and seconds", () => {
    expect(runDuration(makeRun(), now)).toBe("3m 55s");
  });

  it("drops the seconds when they are zero", () => {
    const neat = makeRun({
      runStartedAt: "2026-11-20T10:00:00Z",
      updatedAt: "2026-11-20T10:04:00Z",
    });
    expect(runDuration(neat, now)).toBe("4m");
  });

  it("measures an unfinished run against now", () => {
    const running = makeRun({
      conclusion: null,
      status: "in_progress",
      runStartedAt: "2026-11-20T11:59:20Z",
    });

    expect(runDuration(running, now)).toBe("40s");
  });

  it("is empty when the run never started", () => {
    expect(runDuration(makeRun({ runStartedAt: null }), now)).toBe("");
  });
});

describe("shortSha", () => {
  it("truncates to seven characters", () => {
    expect(shortSha("a1b2c3d4e5f6")).toBe("a1b2c3d");
  });

  it("handles a missing sha", () => {
    expect(shortSha(null)).toBe("");
  });
});
