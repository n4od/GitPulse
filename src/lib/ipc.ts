import { invoke } from "@tauri-apps/api/core";
import { toPulseError } from "@/lib/errors";

/**
 * Single entry point for every backend call.
 *
 * Wrapping `invoke` here means:
 *  - callers get a typed result and a `PulseError` on failure, never a raw throw;
 *  - command names live in one place, so a rename in Rust surfaces as one diff;
 *  - logging / retry policy can be added centrally later without touching call sites.
 */
export async function call<TResult = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<TResult> {
  try {
    return await invoke<TResult>(command, args);
  } catch (cause) {
    throw toPulseError(cause);
  }
}
