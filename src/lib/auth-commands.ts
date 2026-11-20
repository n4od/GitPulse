import type { AuthState } from "@/lib/bindings/AuthState";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { toast } from "@/stores/toasts";

/**
 * Commands the auth UI can issue.
 *
 * Failures are surfaced as toasts here rather than returned, because every
 * caller would do the same thing with them and the authoritative state arrives
 * over `auth://state` anyway.
 */

export async function startSignIn(): Promise<void> {
  try {
    await call("auth_start_device_flow");
  } catch (cause) {
    const error = toPulseError(cause);
    toast("error", "Could not start sign-in", error.message);
  }
}

export async function cancelSignIn(): Promise<void> {
  try {
    await call("auth_cancel_device_flow");
  } catch (cause) {
    toast("error", "Could not cancel sign-in", toPulseError(cause).message);
  }
}

export async function signOut(): Promise<void> {
  try {
    await call("auth_sign_out");
    toast("info", "Signed out");
  } catch (cause) {
    toast("error", "Could not sign out", toPulseError(cause).message);
  }
}

export async function retrySignIn(): Promise<void> {
  try {
    const state = await call<AuthState>("auth_refresh_session");
    if (state.phase !== "authorized") {
      await startSignIn();
    }
  } catch (cause) {
    toast("error", "Could not refresh the session", toPulseError(cause).message);
  }
}

export function openVerificationUri(uri: string): void {
  void call("system_open_url", { url: uri }).catch((cause: unknown) => {
    toast("error", "Could not open the browser", toPulseError(cause).message);
  });
}
