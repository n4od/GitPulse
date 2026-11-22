import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import type { AuthState } from "@/lib/bindings/AuthState";
import { call } from "@/lib/ipc";
import { useAuthStore } from "@/stores/auth";
import { isTauriRuntime } from "@/lib/tauri";

/** Event emitted by the Rust side on every auth transition. */
export const AUTH_STATE_EVENT = "auth://state";

/**
 * Connects the auth store to the backend. Mount exactly once, at the app root.
 *
 * The listener is registered before the initial fetch so a transition that
 * lands during the fetch is not dropped. Both steps are individually guarded:
 * outside a Tauri window (tests, a plain browser) the bridge degrades to the
 * signed-out state instead of breaking the tree.
 */
export function useAuthBridge(): void {
  const queryClient = useQueryClient();
  const setState = useAuthStore((store) => store.setState);
  const setReady = useAuthStore((store) => store.setReady);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    /**
     * Which account the in-memory cache belongs to, so that a change can be told
     * apart from a repeat of the same state.
     */
    let cachedFor: string | null = null;

    /**
     * Signing out discards the account's cache on the Rust side, and switching
     * accounts swaps the database under it. Either way the in-memory copy has to
     * go with it: a card fetched with the previous account's token would
     * otherwise render as though it were this account's — the same mistake
     * whether the account changed by signing out or by switching.
     */
    const apply = (state: AuthState): void => {
      const login = state.phase === "authorized" ? state.session.login : null;

      if (login !== cachedFor) {
        queryClient.clear();
        cachedFor = login;
      }

      setState(state);
    };

    const connect = async (): Promise<void> => {
      if (!isTauriRuntime()) {
        setReady();
        return;
      }

      try {
        const stop = await listen<AuthState>(AUTH_STATE_EVENT, (event) => {
          if (active) apply(event.payload);
        });

        if (active) {
          unlisten = stop;
        } else {
          // Unmounted while the listener was being registered.
          stop();
        }
      } catch (error) {
        console.error("could not subscribe to auth state", error);
      }

      try {
        const initial = await call<AuthState>("auth_get_state");
        if (active) apply(initial);
      } catch (error) {
        // Reading the state failing is not fatal: render the signed-out screen
        // and let the user try to sign in.
        console.error("could not read the initial auth state", error);
      } finally {
        if (active) setReady();
      }
    };

    void connect();

    return () => {
      active = false;
      unlisten?.();
    };
  }, [queryClient, setState, setReady]);
}
