import { beforeEach, describe, expect, it } from "vitest";
import { useUiStore } from "@/stores/ui";

describe("ui store", () => {
  beforeEach(() => {
    localStorage.clear();
    useUiStore.setState({ density: "comfortable", inspectedRepoId: null });
  });

  it("starts comfortable", () => {
    expect(useUiStore.getState().density).toBe("comfortable");
  });

  it("toggles between comfortable and compact", () => {
    useUiStore.getState().toggleDensity();
    expect(useUiStore.getState().density).toBe("compact");

    useUiStore.getState().toggleDensity();
    expect(useUiStore.getState().density).toBe("comfortable");
  });

  it("persists the density preference", () => {
    useUiStore.getState().setDensity("compact");

    expect(localStorage.getItem("gitpulse.ui")).toContain("compact");
  });

  it("does not persist which inspector is open", () => {
    useUiStore.getState().inspect(42);

    // Per-session state: restoring it could reopen the inspector for a
    // repository that is no longer watched.
    expect(localStorage.getItem("gitpulse.ui")).not.toContain("inspectedRepoId");
  });
});
