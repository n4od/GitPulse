import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { LogViewer } from "@/components/runs/LogViewer";
import { useToastStore } from "@/stores/toasts";

// jsdom has no layout, so the virtualizer would render no rows.
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count }: { count: number }) => ({
    getTotalSize: () => count * 18,
    getVirtualItems: () =>
      Array.from({ length: count }, (_, index) => ({
        key: index,
        index,
        start: index * 18,
      })),
    scrollToIndex: vi.fn(),
  }),
}));

describe("LogViewer", () => {
  beforeEach(() => {
    useToastStore.setState({ toasts: [] });
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });
  });

  it("renders the log lines", () => {
    render(<LogViewer download={{ text: "alpha\nbravo\ncharlie", truncated: false }} />);

    expect(screen.getByText("alpha")).toBeInTheDocument();
    expect(screen.getByText("charlie")).toBeInTheDocument();
  });

  it("warns when the download was cut short", () => {
    render(<LogViewer download={{ text: "alpha", truncated: true }} />);

    expect(screen.getByText(/cut short/i)).toBeInTheDocument();
  });

  it("does not warn for a complete log", () => {
    render(<LogViewer download={{ text: "alpha", truncated: false }} />);

    expect(screen.queryByText(/cut short/i)).not.toBeInTheDocument();
  });

  it("counts search matches as the query is typed", async () => {
    render(
      <LogViewer
        download={{ text: "compiling\nrunning\ncompiling again", truncated: false }}
      />,
    );

    expect(screen.getByText("0 matches")).toBeInTheDocument();

    await userEvent.type(screen.getByLabelText(/search the log/i), "compiling");

    expect(screen.getByText("2 matches")).toBeInTheDocument();
  });

  it("uses the singular for one match", async () => {
    render(<LogViewer download={{ text: "only one hit here", truncated: false }} />);

    await userEvent.type(screen.getByLabelText(/search the log/i), "hit");

    expect(screen.getByText("1 match")).toBeInTheDocument();
  });

  it("disables the jump-to-failure button when there is no failure", () => {
    render(<LogViewer download={{ text: "everything fine", truncated: false }} />);

    expect(
      screen.getByRole("button", { name: /jump to the first failure/i }),
    ).toBeDisabled();
  });

  it("enables the jump-to-failure button when the log has one", () => {
    render(<LogViewer download={{ text: "fine\n##[error]boom", truncated: false }} />);

    expect(
      screen.getByRole("button", { name: /jump to the first failure/i }),
    ).toBeEnabled();
  });

  it("copies the whole log", async () => {
    render(<LogViewer download={{ text: "alpha\nbravo", truncated: false }} />);

    await userEvent.click(screen.getByRole("button", { name: /copy the whole log/i }));

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("alpha\nbravo");
    expect(useToastStore.getState().toasts[0]?.title).toBe("Log copied");
  });

  it("reports a clipboard failure instead of throwing", async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockRejectedValue(new Error("denied")) },
    });

    render(<LogViewer download={{ text: "alpha", truncated: false }} />);
    await userEvent.click(screen.getByRole("button", { name: /copy the whole log/i }));

    expect(useToastStore.getState().toasts[0]?.tone).toBe("error");
  });
});
