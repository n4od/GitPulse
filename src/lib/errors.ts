/**
 * Shape of every error crossing the Rust -> JS boundary.
 * Mirrors `AppError` in `src-tauri/src/error.rs` — keep both in sync.
 */
export interface PulseErrorPayload {
  code: string;
  message: string;
  retryable: boolean;
  details?: unknown;
}

export class PulseError extends Error {
  readonly code: string;
  readonly retryable: boolean;
  readonly details: unknown;

  constructor(payload: PulseErrorPayload) {
    super(payload.message);
    this.name = "PulseError";
    this.code = payload.code;
    this.retryable = payload.retryable;
    this.details = payload.details;
  }
}

function isPulseErrorPayload(value: unknown): value is PulseErrorPayload {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate["code"] === "string" &&
    typeof candidate["message"] === "string" &&
    typeof candidate["retryable"] === "boolean"
  );
}

/**
 * Normalises anything thrown by Tauri into a `PulseError`, so components never
 * have to parse error strings or defend against `unknown`.
 */
export function toPulseError(cause: unknown): PulseError {
  if (cause instanceof PulseError) return cause;
  if (isPulseErrorPayload(cause)) return new PulseError(cause);

  const message =
    cause instanceof Error
      ? cause.message
      : typeof cause === "string"
        ? cause
        : "Unknown error";

  return new PulseError({ code: "unknown", message, retryable: false });
}
