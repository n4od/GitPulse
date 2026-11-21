import { create } from "zustand";
import { persist } from "zustand/middleware";

/**
 * How much room a repository card takes.
 *
 * `compact` is aimed at people watching a lot of repositories: the same card,
 * tightened up, so a screen holds more of them.
 */
export type Density = "comfortable" | "compact";

interface UiStore {
  /** The repository whose inspector is open, or null. */
  inspectedRepoId: number | null;
  inspect: (repoId: number | null) => void;
  /** The card size the dashboard renders at. */
  density: Density;
  setDensity: (density: Density) => void;
  toggleDensity: () => void;
}

/**
 * Which repository the inspector is showing, and how dense the cards are.
 *
 * In a store rather than in `Dashboard` state because the command palette and
 * the tray both need to open a repository, and neither is a child of the
 * dashboard.
 *
 * Only `density` is persisted. It is a preference that should survive a
 * restart; `inspectedRepoId` is per-session state, and restoring it would reopen
 * the inspector for a repository that may since have been untracked.
 */
export const useUiStore = create<UiStore>()(
  persist(
    (set) => ({
      inspectedRepoId: null,
      inspect: (repoId) => set({ inspectedRepoId: repoId }),
      density: "comfortable",
      setDensity: (density) => set({ density }),
      toggleDensity: () =>
        set((store) => ({
          density: store.density === "compact" ? "comfortable" : "compact",
        })),
    }),
    {
      name: "gitpulse.ui",
      partialize: (store) => ({ density: store.density }),
    },
  ),
);
