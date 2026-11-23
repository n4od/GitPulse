import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsPanel } from "@/components/settings/SettingsPanel";
import type { NativeSettings } from "@/lib/bindings/NativeSettings";
import type { SystemMetrics } from "@/lib/bindings/SystemMetrics";
import { useLiveStore } from "@/stores/live";
import { useSettingsStore } from "@/stores/settings";
import { useUpdateStore } from "@/stores/update";

/**
 * `vi.mock` is hoisted above the imports, so the fixtures it closes over have to
 * be hoisted too - a plain `const` below would still be in its temporal dead
 * zone when the factory runs.
 */
const { SETTINGS, METRICS } = vi.hoisted(() => ({
  SETTINGS: {
    notificationsEnabled: true,
    quietHours: null,
    autostart: false,
    closeToTray: true,
    checkUpdatesOnLaunch: true,
  } satisfies NativeSettings,
  METRICS: {
    dbSizeMb: 2.5,
    monitoredRepos: 4,
    cachedRuns: BigInt(105),
    budgetAvailable: 30,
    uptimeSecs: 60,
  } satisfies SystemMetrics,
}));

/**
 * Every command the panel knows about, and a throw for anything else.
 *
 * A mock that quietly resolves `undefined` for a command nobody listed hides
 * exactly the kind of wiring bug this panel has; throwing makes the unlisted
 * case the failure.
 */
vi.mock("@/lib/ipc", () => ({
  call: vi.fn((command: string) => {
    switch (command) {
      case "app_version":
        return Promise.resolve("0.8.2");
      case "native_get_settings":
        return Promise.resolve(SETTINGS);
      case "system_metrics":
        return Promise.resolve(METRICS);
      default:
        return Promise.reject(new Error(`unexpected command: ${command}`));
    }
  }),
}));

vi.mock("@/stores/toasts", () => ({ toast: vi.fn() }));

const OFFER = { version: "0.8.3", notes: null, date: null };

function setup(update: Partial<ReturnType<typeof useUpdateStore.getState>> = {}) {
  useUpdateStore.setState({
    version: "0.8.2",
    phase: "idle",
    available: null,
    progress: null,
    error: null,
    retryable: false,
    ...update,
  });

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });

  return render(
    <QueryClientProvider client={client}>
      <SettingsPanel onClose={() => {}} />
    </QueryClientProvider>,
  );
}

describe("SettingsPanel updates", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({ settings: SETTINGS, loading: false, saving: false });
    useLiveStore.setState({ poller: null, rate: null, paused: false });
  });

  it("offers the install when a newer version exists", () => {
    setup({ phase: "available", available: OFFER });

    expect(screen.getByText(/0\.8\.3 is available/)).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /install 0\.8\.3 and restart/i }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("shows a measurable bar while downloading", () => {
    setup({
      phase: "installing",
      available: OFFER,
      progress: {
        downloaded: 2 * 1024 * 1024,
        total: 4 * 1024 * 1024,
        stage: "downloading",
      },
    });

    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
    expect(screen.getByText(/downloading GitPulse 0\.8\.3 - 50%/i)).toBeInTheDocument();
    expect(screen.getByText(/2\.0 MB of 4\.0 MB/)).toBeInTheDocument();
  });

  it("keeps the button on screen while installing", () => {
    // It disappearing under the cursor is most of what made the click look like
    // it did nothing.
    setup({
      phase: "installing",
      available: OFFER,
      progress: { downloaded: 1024, total: 1024, stage: "installing" },
    });

    const button = screen.getByRole("button", { name: /install 0\.8\.3 and restart/i });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-busy", "true");
  });

  it("says the app will restart once the bytes are in", () => {
    setup({
      phase: "installing",
      available: OFFER,
      progress: { downloaded: 1024, total: 1024, stage: "installing" },
    });

    expect(screen.getByText(/installing GitPulse 0\.8\.3/i)).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });

  it("says it is preparing, not downloading, before any bytes move", () => {
    // An install re-checks the manifest first, and that request can fail - which
    // is exactly what happened the first time this was clicked. Calling that
    // moment "downloading" would be a lie about a download that never started.
    setup({
      phase: "installing",
      available: OFFER,
      progress: { downloaded: 0, total: null, stage: "preparing" },
    });

    expect(
      screen.getByText(/preparing to install GitPulse 0\.8\.3/i),
    ).toBeInTheDocument();
    expect(screen.queryByText(/downloading/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("draws no bar when the server declared no size", () => {
    // An empty bar and a full bar would both be lies; the text carries it.
    setup({
      phase: "installing",
      available: OFFER,
      progress: { downloaded: 0, total: null, stage: "downloading" },
    });

    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
    expect(screen.getByText(/downloading GitPulse 0\.8\.3/i)).toBeInTheDocument();
  });
});
