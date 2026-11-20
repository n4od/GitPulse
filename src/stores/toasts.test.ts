import { beforeEach, describe, expect, it } from "vitest";
import { toast, useToastStore } from "@/stores/toasts";

describe("toast store", () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
  });

  it("starts empty", () => {
    expect(useToastStore.getState().toasts).toEqual([]);
  });

  it("pushes a toast and returns its id", () => {
    const id = toast("success", "Re-run queued");

    const { toasts } = useToastStore.getState();
    expect(toasts).toHaveLength(1);
    expect(toasts[0]?.id).toBe(id);
    expect(toasts[0]?.title).toBe("Re-run queued");
  });

  it("omits description when none is given", () => {
    toast("info", "Paused");
    expect(useToastStore.getState().toasts[0]).not.toHaveProperty("description");
  });

  it("keeps a description when one is given", () => {
    toast("success", "Re-run queued", "acme/api · deploy.yml");
    expect(useToastStore.getState().toasts[0]?.description).toBe("acme/api · deploy.yml");
  });

  it("dismisses by id without touching the others", () => {
    const first = toast("info", "One");
    toast("info", "Two");

    useToastStore.getState().dismiss(first);

    const { toasts } = useToastStore.getState();
    expect(toasts).toHaveLength(1);
    expect(toasts[0]?.title).toBe("Two");
  });

  it("caps the queue so a failure storm cannot fill the screen", () => {
    for (let index = 0; index < 6; index += 1) {
      toast("error", `Failure ${index}`);
    }

    const { toasts } = useToastStore.getState();
    expect(toasts).toHaveLength(3);
    expect(toasts.map((item) => item.title)).toEqual([
      "Failure 3",
      "Failure 4",
      "Failure 5",
    ]);
  });
});
