import { create } from "zustand";
import type { AuthState } from "@/lib/bindings/AuthState";
import type { AuthSession } from "@/lib/bindings/AuthSession";

interface AuthStore {
  state: AuthState;
  /** False until the initial `auth_get_state` round-trip has settled. */
  ready: boolean;
  setState: (state: AuthState) => void;
  setReady: () => void;
}

/**
 * Auth is a store rather than a hook because three separate places need it —
 * the title bar, the landing route and (later) the tray — and each one
 * subscribing to `auth://state` independently would mean three listeners
 * processing every event.
 */
export const useAuthStore = create<AuthStore>((set) => ({
  state: { phase: "signedOut" },
  ready: false,
  setState: (state) => set({ state }),
  setReady: () => set({ ready: true }),
}));

/** The signed-in session, or null. */
export function useSession(): AuthSession | null {
  return useAuthStore((store) =>
    store.state.phase === "authorized" ? store.state.session : null,
  );
}

/** True while a sign-in is in progress and can be cancelled. */
export function useSigningIn(): boolean {
  return useAuthStore(
    (store) =>
      store.state.phase === "requestingCode" ||
      store.state.phase === "awaitingUser" ||
      store.state.phase === "polling",
  );
}
