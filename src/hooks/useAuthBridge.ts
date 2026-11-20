import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { AuthState } from "@/lib/bindings/AuthState";
import { call } from "@/lib/ipc";
import { useAuthStore } from "@/stores/auth";

/** Event emitted by the Rust side on every auth transition. */
export const AUTH_STATE_EVENT = "auth://state";

/**
 * True when the page is running inside the Tauri webview.
 *
 * Outside it — `vite dev` in a browser, or a component test — there is no IPC
 * to talk to, so the bridge stays idle instead of logging a caught error on
 * every mount.
 */
function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Connects the auth store to the backend. Mount exactly once, at the app root.
 *
 * The listener is registered before the initial fetch so a transition that
 * lands during the fetch is not dropped. Both steps are individually guarded:
 * outside a Tauri window (tests, a plain browser) the bridge degrades to the
 * signed-out state instead of breaking the tree.
 */
export function useAuthBridge(): void {
  const setState = useAuthStore((store) => store.setState);
  const setReady = useAuthStore((store) => store.setReady);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    const connect = async (): Promise<void> => {
      if (!isTauriRuntime()) {
        setReady();
        return;
      }

      try {
        const stop = await listen<AuthState>(AUTH_STATE_EVENT, (event) => {
          if (active) setState(event.payload);
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
        if (active) setState(initial);
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
  }, [setState, setReady]);
}
