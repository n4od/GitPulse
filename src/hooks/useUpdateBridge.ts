import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UpdateInfo } from "@/lib/bindings/UpdateInfo";
import type { UpdateProgress } from "@/lib/bindings/UpdateProgress";
import { call } from "@/lib/ipc";
import { isTauriRuntime } from "@/lib/tauri";
import { useUpdateStore } from "@/stores/update";

/** Event emitted by the Rust side when a check finds a newer release. */
export const UPDATE_AVAILABLE_EVENT = "update://available";

/** Event emitted while an install downloads, and once when it installs. */
export const UPDATE_PROGRESS_EVENT = "update://progress";

/**
 * Connects the update store to the backend. Mount exactly once, at the app root.
 *
 * The check that matters runs in Rust, during bootstrap: the setting that gates
 * it is read before the window exists, and a build published while the app was
 * closed is exactly what it is for. The listeners are registered *before* the
 * catch-up fetch, so a check that finished while the window was still loading is
 * not dropped - the same ordering `useAuthBridge` uses, and for the same reason.
 */
export function useUpdateBridge(): void {
  const announce = useUpdateStore((store) => store.announce);
  const setProgress = useUpdateStore((store) => store.setProgress);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let active = true;
    const stops: Array<() => void> = [];

    const subscribe = async <T>(
      event: string,
      handler: (payload: T) => void,
    ): Promise<void> => {
      try {
        const unsubscribe = await listen<T>(event, (message) => {
          if (active) handler(message.payload);
        });

        if (active) {
          stops.push(unsubscribe);
        } else {
          // Unmounted while the listener was being registered.
          unsubscribe();
        }
      } catch (error) {
        console.error(`could not subscribe to ${event}`, error);
      }
    };

    const connect = async (): Promise<void> => {
      await subscribe<UpdateInfo>(UPDATE_AVAILABLE_EVENT, announce);
      await subscribe<UpdateProgress>(UPDATE_PROGRESS_EVENT, setProgress);

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
      for (const stop of stops) stop();
    };
  }, [announce, setProgress]);
}
