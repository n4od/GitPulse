import { describe, expect, it } from "vitest";
import { mapRunStatus, worstStatus, type StatusPresentation } from "@/lib/status";

interface Row {
  status: string | null;
  conclusion: string | null;
  key: StatusPresentation["key"];
  label: string;
  tone: StatusPresentation["tone"];
  pulse: boolean;
}

/**
 * Mirrors the roadmap's status table one-to-one. If a row is added to
 * `lib/status.ts`, it must be added here — this test is the contract.
 */
const TABLE: Row[] = [
  {
    status: "queued",
    conclusion: null,
    key: "queued",
    label: "Queued",
    tone: "neutral",
    pulse: false,
  },
  {
    status: "waiting",
    conclusion: null,
    key: "queued",
    label: "Queued",
    tone: "neutral",
    pulse: false,
  },
  {
    status: "pending",
    conclusion: null,
    key: "queued",
    label: "Queued",
    tone: "neutral",
    pulse: false,
  },
  {
    status: "requested",
    conclusion: null,
    key: "queued",
    label: "Queued",
    tone: "neutral",
    pulse: false,
  },
  {
    status: "in_progress",
    conclusion: null,
    key: "running",
    label: "Running",
    tone: "running",
    pulse: true,
  },
  {
    status: "completed",
    conclusion: "success",
    key: "success",
    label: "Success",
    tone: "success",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "failure",
    key: "failed",
    label: "Failed",
    tone: "failure",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "startup_failure",
    key: "failed",
    label: "Failed",
    tone: "failure",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "cancelled",
    key: "cancelled",
    label: "Cancelled",
    tone: "muted",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "timed_out",
    key: "timed_out",
    label: "Timed out",
    tone: "failure",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "action_required",
    key: "action_required",
    label: "Action needed",
    tone: "warn",
    pulse: true,
  },
  {
    status: "completed",
    conclusion: "neutral",
    key: "neutral",
    label: "Neutral",
    tone: "neutral",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "skipped",
    key: "neutral",
    label: "Neutral",
    tone: "neutral",
    pulse: false,
  },
  {
    status: "completed",
    conclusion: "stale",
    key: "neutral",
    label: "Neutral",
    tone: "neutral",
    pulse: false,
  },
];

describe("mapRunStatus", () => {
  it.each(TABLE)(
    "maps $status/$conclusion to $key",
    ({ status, conclusion, key, label, tone, pulse }) => {
      const result = mapRunStatus(status, conclusion);
      expect(result).toMatchObject({ key, label, tone, pulse });
    },
  );

  it("normalises casing and whitespace", () => {
    expect(mapRunStatus(" In_Progress ", null).key).toBe("running");
    expect(mapRunStatus("completed", " FAILURE ").key).toBe("failed");
  });

  it("treats a conclusion as authoritative even when status is stale", () => {
    expect(mapRunStatus("in_progress", "success").key).toBe("success");
    expect(mapRunStatus("queued", "failure").key).toBe("failed");
  });

  it("does not invent a success for a completed run without a conclusion", () => {
    expect(mapRunStatus("completed", null).key).toBe("unknown");
  });

  it("falls back to unknown for unrecognised input", () => {
    expect(mapRunStatus("teleported", null).key).toBe("unknown");
    expect(mapRunStatus(null, null).key).toBe("unknown");
    expect(mapRunStatus(undefined, undefined).key).toBe("unknown");
    expect(mapRunStatus("completed", "quantum_superposition").key).toBe("unknown");
  });
});

describe("worstStatus", () => {
  const status = (s: string, c: string | null = null) => mapRunStatus(s, c);

  it("returns null when there is nothing to show", () => {
    expect(worstStatus([])).toBeNull();
  });

  it("prefers failure over everything else", () => {
    const result = worstStatus([
      status("completed", "success"),
      status("in_progress"),
      status("completed", "failure"),
      status("completed", "action_required"),
    ]);
    expect(result?.key).toBe("failed");
  });

  it("prefers action_required over a running job", () => {
    const result = worstStatus([
      status("in_progress"),
      status("completed", "action_required"),
      status("completed", "success"),
    ]);
    expect(result?.key).toBe("action_required");
  });

  it("prefers running over queued and success", () => {
    const result = worstStatus([
      status("queued"),
      status("in_progress"),
      status("completed", "success"),
    ]);
    expect(result?.key).toBe("running");
  });

  it("returns the single entry when given one", () => {
    expect(worstStatus([status("completed", "cancelled")])?.key).toBe("cancelled");
  });
});
