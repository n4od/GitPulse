/**
 * The canonical mapping from GitHub's run vocabulary to GitPulse's status badges.
 *
 * This module is the single source of truth for status semantics: the UI, the
 * tray icon and the notification rules all read from here. Adding a conclusion
 * means adding one row to `PRESENTATIONS` and one test case.
 */

export type StatusTone = "neutral" | "running" | "success" | "failure" | "warn" | "muted";

export type StatusKey =
  | "queued"
  | "running"
  | "success"
  | "failed"
  | "cancelled"
  | "timed_out"
  | "action_required"
  | "neutral"
  | "unknown";

export interface StatusPresentation {
  key: StatusKey;
  label: string;
  tone: StatusTone;
  /** Whether the indicator should pulse (only meaningful for transient states). */
  pulse: boolean;
  /** Higher wins when collapsing a repo's runs into one badge. */
  severity: number;
}

const PRESENTATIONS = {
  queued: { key: "queued", label: "Queued", tone: "neutral", pulse: false, severity: 40 },
  running: {
    key: "running",
    label: "Running",
    tone: "running",
    pulse: true,
    severity: 60,
  },
  success: {
    key: "success",
    label: "Success",
    tone: "success",
    pulse: false,
    severity: 10,
  },
  failed: {
    key: "failed",
    label: "Failed",
    tone: "failure",
    pulse: false,
    severity: 100,
  },
  cancelled: {
    key: "cancelled",
    label: "Cancelled",
    tone: "muted",
    pulse: false,
    severity: 20,
  },
  timed_out: {
    key: "timed_out",
    label: "Timed out",
    tone: "failure",
    pulse: false,
    severity: 100,
  },
  action_required: {
    key: "action_required",
    label: "Action needed",
    tone: "warn",
    pulse: true,
    severity: 80,
  },
  neutral: {
    key: "neutral",
    label: "Neutral",
    tone: "neutral",
    pulse: false,
    severity: 5,
  },
  unknown: {
    key: "unknown",
    label: "Unknown",
    tone: "neutral",
    pulse: false,
    severity: 0,
  },
} as const satisfies Record<StatusKey, StatusPresentation>;

function normalise(value: string | null | undefined): string {
  return value?.trim().toLowerCase() ?? "";
}

/**
 * Resolves a conclusion once a run has finished. `startup_failure` is a real
 * GitHub conclusion that the roadmap table omits; it is treated as a failure
 * because that is what the user sees.
 */
function fromConclusion(conclusion: string): StatusPresentation {
  switch (conclusion) {
    case "success":
      return PRESENTATIONS.success;
    case "failure":
    case "startup_failure":
      return PRESENTATIONS.failed;
    case "cancelled":
      return PRESENTATIONS.cancelled;
    case "timed_out":
      return PRESENTATIONS.timed_out;
    case "action_required":
      return PRESENTATIONS.action_required;
    case "neutral":
    case "skipped":
    case "stale":
      return PRESENTATIONS.neutral;
    default:
      return PRESENTATIONS.unknown;
  }
}

/**
 * Maps a GitHub Actions `status` + `conclusion` pair to a badge.
 *
 * `conclusion` wins when present: GitHub reports `status: "completed"` with a
 * conclusion, but webhook payloads in flight can carry a conclusion while the
 * status field is still stale.
 */
export function mapRunStatus(
  status: string | null | undefined,
  conclusion: string | null | undefined,
): StatusPresentation {
  const normalisedStatus = normalise(status);
  const normalisedConclusion = normalise(conclusion);

  if (normalisedConclusion !== "") {
    return fromConclusion(normalisedConclusion);
  }

  switch (normalisedStatus) {
    case "queued":
    case "waiting":
    case "pending":
    case "requested":
      return PRESENTATIONS.queued;
    case "in_progress":
      return PRESENTATIONS.running;
    case "completed":
      // Completed without a conclusion is malformed data; say so instead of
      // inventing a success.
      return PRESENTATIONS.unknown;
    default:
      return PRESENTATIONS.unknown;
  }
}

/**
 * Collapses many runs into the single badge a repository card shows.
 * Ordering: failure > action_required > in_progress > queued > success.
 * Returns `null` for an empty list so the caller can render "no runs yet".
 */
export function worstStatus(
  presentations: readonly StatusPresentation[],
): StatusPresentation | null {
  let worst: StatusPresentation | null = null;

  for (const presentation of presentations) {
    if (worst === null || presentation.severity > worst.severity) {
      worst = presentation;
    }
  }

  return worst;
}
