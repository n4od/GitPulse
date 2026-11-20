import { create } from "zustand";
import type { NativeSettings } from "@/lib/bindings/NativeSettings";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { toast } from "@/stores/toasts";

interface SettingsStore {
  settings: NativeSettings | null;
  loading: boolean;
  saving: boolean;
  /** Reads the settings from the backend. Safe to call more than once. */
  load: () => Promise<void>;
  /**
   * Applies a change optimistically and rolls it back if the backend refuses.
   *
   * A checkbox that only moves after a round trip feels broken; one that moves
   * and silently stays wrong is worse. The previous value is restored and the
   * user is told why.
   */
  update: (patch: Partial<NativeSettings>) => Promise<void>;
}

const DEFAULTS: NativeSettings = {
  notificationsEnabled: true,
  quietHours: null,
  autostart: false,
  closeToTray: true,
};

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: null,
  loading: false,
  saving: false,

  load: async () => {
    set({ loading: true });

    try {
      const settings = await call<NativeSettings>("native_get_settings");
      set({ settings });
    } catch (cause) {
      // Fall back to the documented defaults rather than showing nothing.
      set({ settings: DEFAULTS });
      toast("error", "Could not read settings", toPulseError(cause).message);
    } finally {
      set({ loading: false });
    }
  },

  update: async (patch) => {
    const previous = get().settings;
    if (!previous) return;

    const next = { ...previous, ...patch };
    set({ settings: next, saving: true });

    try {
      const saved = await call<NativeSettings>("native_set_settings", { settings: next });
      set({ settings: saved });
    } catch (cause) {
      set({ settings: previous });
      toast("error", "Could not save that setting", toPulseError(cause).message);
    } finally {
      set({ saving: false });
    }
  },
}));
