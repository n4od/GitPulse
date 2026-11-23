import { create } from "zustand";
import type { UpdateInfo } from "@/lib/bindings/UpdateInfo";
import type { UpdateProgress } from "@/lib/bindings/UpdateProgress";
import { toPulseError } from "@/lib/errors";
import { call } from "@/lib/ipc";
import { toast } from "@/stores/toasts";

/** How far a check, or an install, has got. */
export type UpdatePhase = "idle" | "checking" | "current" | "available" | "installing";

interface UpdateStore {
  /** The version this build reports - the same number the updater compares. */
  version: string | null;
  phase: UpdatePhase;
  /** Set only while `phase` is "available". */
  available: UpdateInfo | null;
  /**
   * Bytes in flight, set only while `phase` is "installing".
   *
   * The install ends by taking the process away, so this is the last thing the
   * user sees - and it is the only moving part in a wait that otherwise looks
   * like a click that did nothing.
   */
  progress: UpdateProgress | null;
  /** Why the last attempt failed, if it did. */
  error: string | null;
  /** Whether repeating the failed call is worth offering at all. */
  retryable: boolean;

  loadVersion: () => Promise<void>;
  /**
   * Records an update found by the startup check or by a manual one.
   *
   * Both paths come through here so the user hears about a given version once,
   * no matter which of them noticed it.
   */
  announce: (info: UpdateInfo) => void;
  /** Records install progress reported by the backend. */
  setProgress: (progress: UpdateProgress) => void;
  check: () => Promise<void>;
  install: () => Promise<void>;
}

export const useUpdateStore = create<UpdateStore>((set, get) => ({
  version: null,
  phase: "idle",
  available: null,
  progress: null,
  error: null,
  retryable: false,

  loadVersion: async () => {
    try {
      set({ version: await call<string>("app_version") });
    } catch {
      // The version line is information, not function. If it cannot be read the
      // panel shows a dash; that is not worth a toast, and not worth a retry.
    }
  },

  announce: (info) => {
    const { available, phase } = get();

    // The event and the startup catch-up read can both deliver the same update,
    // depending on which wins the race. The user should be told once.
    if (phase === "available" && available?.version === info.version) return;

    set({ phase: "available", available: info, error: null, progress: null });
    toast(
      "info",
      `GitPulse ${info.version} is available`,
      "Open Settings to install it.",
    );
  },

  setProgress: (progress) => {
    // Dropped unless an install is running. The backend emits on its own
    // schedule, and a late event must not resurrect the bar for an install that
    // has already finished - or failed and put the button back.
    if (get().phase !== "installing") return;
    set({ progress });
  },

  check: async () => {
    set({ phase: "checking", error: null, retryable: false, progress: null });

    try {
      const available = await call<UpdateInfo | null>("update_check");

      if (available) {
        get().announce(available);
        return;
      }

      set({ phase: "current", available: null });
      const { version } = get();
      toast(
        "success",
        "GitPulse is up to date",
        version ? `Version ${version}` : undefined,
      );
    } catch (cause) {
      const error = toPulseError(cause);
      set({
        phase: "idle",
        available: null,
        error: error.message,
        retryable: error.retryable,
      });
      toast("error", "Could not check for updates", error.message);
    }
  },

  install: async () => {
    const { available } = get();

    set({
      phase: "installing",
      error: null,
      // Seeded here rather than waiting for the first event, so the click is
      // visibly answered even before the IPC round trip lands. Nothing has been
      // fetched yet at this point, which is what "preparing" says.
      progress: { downloaded: 0, total: null, stage: "preparing" },
    });

    if (available) {
      toast(
        "info",
        `Downloading GitPulse ${available.version}`,
        "GitPulse will restart when it is ready.",
      );
    }

    try {
      await call("update_install");
      // Reaching here means the process did not go away, which is not what a
      // successful install looks like: on Windows the installer takes the app
      // down during the call, and everywhere else the backend restarts it. So
      // put the button back rather than pretend something happened.
      set({ phase: "available", progress: null });
    } catch (cause) {
      const error = toPulseError(cause);
      set({
        phase: "available",
        progress: null,
        error: error.message,
        retryable: error.retryable,
      });
      toast("error", "Could not install the update", error.message);
    }
  },
}));
