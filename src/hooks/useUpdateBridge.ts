import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UpdateInfo } from "@/lib/bindings/UpdateInfo";
import { call } from "@/lib/ipc";
import { isTauriRuntime } from "@/lib/tauri";
import { useUpdateStore } from "@/stores/update";

/** Event emitted by the Rust side when a check finds a newer release. */
export const UPDATE_AVAILABLE_EVENT = "update://available";

/**
 * Connects the update store to the backend. Mount exactly once, at the app root.
 *
 * The check that matters runs in Rust, during bootstrap: the setting that gates
 * it is read before the window exists, and a build published while the app was
 * closed is exactly what it is for. The listener is registered *before* the
 * catch-up fetch, so a check that finished while the window was still loading is
 * not dropped - the same ordering `useAuthBridge` uses, and for the same reason.
 */
export function useUpdateBridge(): void {
  const announce = useUpdateStore((store) => store.announce);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let active = true;
    let stop: (() => void) | undefined;

    const connect = async (): Promise<void> => {
      try {
        const unsubscribe = await listen<UpdateInfo>(UPDATE_AVAILABLE_EVENT, (event) => {
          if (active) announce(event.payload);
        });

        if (active) {
          stop = unsubscribe;
        } else {
          // Unmounted while the listener was being registered.
          unsubscribe();
        }
      } catch (error) {
        console.error("could not subscribe to update events", error);
      }

      try {
        const found = await call<UpdateInfo | null>("update_pending");
        if (active && found) announce(found);
      } catch (error) {
        // Not fatal: a manual check still works, and the version line is
        // decoration. Losing the startup notice is the only cost.
        console.error("could not read the last update check", error);
      }
    };

    void connect();

    return () => {
      active = false;
      stop?.();
    };
  }, [announce]);
}
