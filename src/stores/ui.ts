import { create } from "zustand";

interface UiStore {
  /** The repository whose inspector is open, or null. */
  inspectedRepoId: number | null;
  inspect: (repoId: number | null) => void;
}

/**
 * Which repository the inspector is showing.
 *
 * In a store rather than in `Dashboard` state because the command palette and
 * the tray both need to open a repository, and neither is a child of the
 * dashboard.
 */
export const useUiStore = create<UiStore>((set) => ({
  inspectedRepoId: null,
  inspect: (repoId) => set({ inspectedRepoId: repoId }),
}));
