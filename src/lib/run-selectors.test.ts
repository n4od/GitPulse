import { describe, expect, it } from "vitest";
import { makeCard, makeRun } from "@/test/fixtures";
import {
  firstFailureLine,
  matchingLines,
  optimisticallySetRunStatus,
  toLines,
} from "@/lib/run-selectors";

describe("optimisticallySetRunStatus", () => {
  it("changes the target run's status and clears its conclusion", () => {
    const card = makeCard({
      recentRuns: [makeRun({ id: 1, status: "completed", conclusion: "failure" })],
    });

    const next = optimisticallySetRunStatus(card, 1, "queued");

    expect(next.recentRuns[0]?.status).toBe("queued");
    expect(next.recentRuns[0]?.conclusion).toBeNull();
  });

  it("leaves the other runs alone", () => {
    const card = makeCard({
      recentRuns: [
        makeRun({ id: 1, status: "completed", conclusion: "failure" }),
        makeRun({ id: 2, status: "completed", conclusion: "success" }),
      ],
    });

    const next = optimisticallySetRunStatus(card, 1, "queued");

    expect(next.recentRuns[1]?.conclusion).toBe("success");
  });

  it("is a no-op for a run that is not on the card", () => {
    const card = makeCard({ recentRuns: [makeRun({ id: 1 })] });

    expect(optimisticallySetRunStatus(card, 99, "queued").recentRuns[0]?.status).toBe(
      card.recentRuns[0]?.status,
    );
  });

  it("does not mutate the card it was given", () => {
    const card = makeCard({ recentRuns: [makeRun({ id: 1, status: "completed" })] });

    optimisticallySetRunStatus(card, 1, "queued");

    expect(card.recentRuns[0]?.status).toBe("completed");
  });
});

describe("toLines", () => {
  it("returns nothing for an empty log", () => {
    expect(toLines("")).toEqual([]);
  });

  it("splits on newlines", () => {
    expect(toLines("one\ntwo")).toEqual(["one", "two"]);
  });

  it("does not invent a blank last line for a trailing newline", () => {
    expect(toLines("one\ntwo\n")).toEqual(["one", "two"]);
  });

  it("keeps genuinely blank interior lines", () => {
    expect(toLines("one\n\ntwo")).toEqual(["one", "", "two"]);
  });
});

describe("firstFailureLine", () => {
  it("finds the workflow error marker", () => {
    const lines = ["setup", "building", "##[error]Process completed with exit code 1"];

    expect(firstFailureLine(lines)).toBe(2);
  });

  it("finds the first marker, not the last", () => {
    const lines = ["##[error]first", "##[error]second"];

    expect(firstFailureLine(lines)).toBe(0);
  });

  it("recognises a plain failure line", () => {
    expect(firstFailureLine(["ok", "1 test FAILED"])).toBe(1);
  });

  it("returns -1 when nothing looks like a failure", () => {
    expect(firstFailureLine(["all good", "done"])).toBe(-1);
  });

  it("returns -1 for an empty log", () => {
    expect(firstFailureLine([])).toBe(-1);
  });
});

describe("matchingLines", () => {
  const lines = ["Compiling pulse", "warning: unused", "Compiling other"];

  it("matches case-insensitively", () => {
    expect(matchingLines(lines, "COMPILING")).toEqual([0, 2]);
  });

  it("ignores surrounding whitespace", () => {
    expect(matchingLines(lines, "  warning  ")).toEqual([1]);
  });

  it("returns nothing for an empty query", () => {
    expect(matchingLines(lines, "")).toEqual([]);
  });

  it("returns nothing when there is no match", () => {
    expect(matchingLines(lines, "nope")).toEqual([]);
  });
});
