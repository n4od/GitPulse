import { beforeEach, describe, expect, it, vi } from "vitest";
import { useUpdateStore } from "@/stores/update";

const { callMock } = vi.hoisted(() => ({ callMock: vi.fn() }));

vi.mock("@/lib/ipc", () => ({ call: callMock }));
vi.mock("@/stores/toasts", () => ({ toast: vi.fn() }));

function reset() {
  useUpdateStore.setState({
    version: "0.8.2",
    phase: "idle",
    available: null,
    progress: null,
    error: null,
    retryable: false,
  });
}

const OFFER = { version: "0.8.3", notes: null, date: null };

describe("update store", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    reset();
  });

  it("answers the click before the first byte arrives", async () => {
    // The whole complaint: pressing "Install and restart" looked like nothing
    // had happened. The bar is seeded from the click itself, not from the first
    // progress event - which on a fast connection may never be seen.
    let settle: (() => void) | undefined;
    callMock.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          settle = resolve;
        }),
    );

    useUpdateStore.setState({ phase: "available", available: OFFER });
    const pending = useUpdateStore.getState().install();

    expect(useUpdateStore.getState().phase).toBe("installing");
    expect(useUpdateStore.getState().progress).toEqual({
      downloaded: 0,
      total: null,
      stage: "preparing",
    });

    settle?.();
    await pending;
  });

  it("records progress reported by the backend while installing", () => {
    useUpdateStore.setState({ phase: "installing" });

    useUpdateStore
      .getState()
      .setProgress({ downloaded: 512, total: 1024, stage: "downloading" });

    expect(useUpdateStore.getState().progress?.downloaded).toBe(512);
  });

  it("ignores progress when no install is running", () => {
    // A late event must not resurrect the bar over a finished or failed
    // install, where the offer is showing again.
    useUpdateStore.setState({ phase: "available", available: OFFER });

    useUpdateStore
      .getState()
      .setProgress({ downloaded: 512, total: 1024, stage: "downloading" });

    expect(useUpdateStore.getState().progress).toBeNull();
  });

  it("puts the offer back when the install fails", async () => {
    callMock.mockRejectedValue(new Error("the signature did not match"));
    useUpdateStore.setState({ phase: "available", available: OFFER });

    await useUpdateStore.getState().install();

    const state = useUpdateStore.getState();
    expect(state.phase).toBe("available");
    expect(state.progress).toBeNull();
    expect(state.error).toBeTruthy();
  });

  it("treats a returned install as one that did not happen", async () => {
    // On Windows the installer takes the process down during the call, so
    // resolving means the install did *not* happen. Claiming success would leave
    // an old binary running behind a UI that says otherwise.
    callMock.mockResolvedValue(undefined);
    useUpdateStore.setState({ phase: "available", available: OFFER });

    await useUpdateStore.getState().install();

    const state = useUpdateStore.getState();
    expect(state.phase).toBe("available");
    expect(state.progress).toBeNull();
    expect(state.error).toBeNull();
  });

  it("clears a stale bar when a fresh check finds nothing", async () => {
    callMock.mockResolvedValue(null);
    useUpdateStore.setState({
      phase: "installing",
      progress: { downloaded: 1, total: 2, stage: "downloading" },
    });

    await useUpdateStore.getState().check();

    const state = useUpdateStore.getState();
    expect(state.phase).toBe("current");
    expect(state.progress).toBeNull();
  });
});
