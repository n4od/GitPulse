import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CommandPalette } from "@/components/shell/CommandPalette";
import { makeCard, makeRepo } from "@/test/fixtures";

function setup(overrides: Partial<Parameters<typeof CommandPalette>[0]> = {}) {
  const handlers = {
    onClose: vi.fn(),
    onOpenRepo: vi.fn(),
    onAction: vi.fn(),
  };

  render(
    <CommandPalette
      cards={[
        makeCard({ repo: makeRepo({ id: 1, fullName: "acme/api" }) }),
        makeCard({ repo: makeRepo({ id: 2, fullName: "acme/website" }) }),
      ]}
      {...handlers}
      {...overrides}
    />,
  );

  return handlers;
}

describe("CommandPalette", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("lists the actions when nothing has been typed", () => {
    setup();

    expect(screen.getByText("Open settings")).toBeInTheDocument();
    expect(screen.getByText("Sign out of GitHub")).toBeInTheDocument();
  });

  it("focuses the search field on open, so typing works immediately", () => {
    setup();

    expect(screen.getByLabelText(/search repositories and actions/i)).toHaveFocus();
  });

  it("finds a repository by a subsequence", async () => {
    setup();

    await userEvent.type(
      screen.getByLabelText(/search repositories and actions/i),
      "acapi",
    );

    expect(screen.getByText("acme/api")).toBeInTheDocument();
  });

  it("opens the highlighted repository on Enter", async () => {
    const handlers = setup();

    await userEvent.type(
      screen.getByLabelText(/search repositories and actions/i),
      "website",
    );
    await userEvent.keyboard("{Enter}");

    expect(handlers.onOpenRepo).toHaveBeenCalledWith(2);
    expect(handlers.onClose).toHaveBeenCalled();
  });

  it("runs an action on Enter", async () => {
    const handlers = setup();

    await userEvent.type(
      screen.getByLabelText(/search repositories and actions/i),
      "settings",
    );
    await userEvent.keyboard("{Enter}");

    expect(handlers.onAction).toHaveBeenCalledWith("settings");
  });

  it("moves the selection with the arrow keys before accepting", async () => {
    const handlers = setup();

    await userEvent.type(
      screen.getByLabelText(/search repositories and actions/i),
      "acme",
    );
    // Two repositories match; the second should win after one ArrowDown.
    await userEvent.keyboard("{ArrowDown}{Enter}");

    expect(handlers.onOpenRepo).toHaveBeenCalledTimes(1);
  });

  it("says so when nothing matches", async () => {
    setup();

    await userEvent.type(
      screen.getByLabelText(/search repositories and actions/i),
      "zzzzzz",
    );

    expect(screen.getByText("Nothing matches.")).toBeInTheDocument();
  });
});
