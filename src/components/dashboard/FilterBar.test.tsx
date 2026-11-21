import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FilterBar, type FilterBarProps } from "@/components/dashboard/FilterBar";

function setup(overrides: Partial<FilterBarProps> = {}) {
  const props: FilterBarProps = {
    query: "",
    filter: "all",
    monitoredCount: 3,
    mineOnly: false,
    canFilterMine: true,
    density: "comfortable",
    onQueryChange: vi.fn(),
    onFilterChange: vi.fn(),
    onMineChange: vi.fn(),
    onToggleDensity: vi.fn(),
    onAddRepositories: vi.fn(),
    onRefreshAll: vi.fn(),
    refreshing: false,
    ...overrides,
  };

  render(<FilterBar {...props} />);
  return props;
}

describe("FilterBar", () => {
  it("asks for the mine filter to be turned on", async () => {
    const user = userEvent.setup();
    const props = setup();

    await user.click(screen.getByRole("button", { name: "Mine" }));

    expect(props.onMineChange).toHaveBeenCalledWith(true);
  });

  it("marks the mine filter as pressed when it is on", () => {
    setup({ mineOnly: true });

    expect(screen.getByRole("button", { name: "Mine" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("hides the mine filter when nobody is signed in", () => {
    setup({ canFilterMine: false });

    expect(screen.queryByRole("button", { name: "Mine" })).not.toBeInTheDocument();
  });

  it("offers to switch to compact cards", async () => {
    const user = userEvent.setup();
    const props = setup();

    await user.click(screen.getByRole("button", { name: "Switch to compact cards" }));

    expect(props.onToggleDensity).toHaveBeenCalledTimes(1);
  });

  it("offers to switch back when the cards are already compact", () => {
    setup({ density: "compact" });

    expect(
      screen.getByRole("button", { name: "Switch to comfortable cards" }),
    ).toHaveAttribute("aria-pressed", "true");
  });

  it("reports the search term", () => {
    const props = setup();

    fireEvent.change(screen.getByRole("searchbox", { name: "Search repositories" }), {
      target: { value: "api" },
    });

    expect(props.onQueryChange).toHaveBeenCalledWith("api");
  });

  it("offers a way back into the repository picker", async () => {
    const user = userEvent.setup();
    const props = setup();

    await user.click(screen.getByRole("button", { name: "Add repositories" }));

    expect(props.onAddRepositories).toHaveBeenCalledTimes(1);
  });
});
