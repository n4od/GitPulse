import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "@/components/primitives/Button";
import { StatusBadge } from "@/components/status/StatusBadge";

describe("Button", () => {
  it("defaults to type=button so it never submits a form by accident", () => {
    render(<Button>Save</Button>);
    expect(screen.getByRole("button", { name: "Save" })).toHaveAttribute(
      "type",
      "button",
    );
  });

  it("calls the handler once per click", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Run</Button>);

    await userEvent.click(screen.getByRole("button", { name: "Run" }));

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("is not clickable while loading", async () => {
    const onClick = vi.fn();
    render(
      <Button loading onClick={onClick}>
        Saving
      </Button>,
    );

    const button = screen.getByRole("button", { name: "Saving" });
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("aria-busy", "true");

    await userEvent.click(button);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("is reachable and activatable by keyboard", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Keyboard</Button>);

    await userEvent.tab();
    expect(screen.getByRole("button", { name: "Keyboard" })).toHaveFocus();

    await userEvent.keyboard("{Enter}");
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe("StatusBadge", () => {
  it("renders the mapped label, not the raw GitHub vocabulary", () => {
    render(<StatusBadge status="completed" conclusion="timed_out" />);
    expect(screen.getByText("Timed out")).toBeInTheDocument();
  });

  it("exposes the raw pair as a tooltip title for debugging", () => {
    render(<StatusBadge status="in_progress" conclusion={null} />);
    expect(screen.getByText("Running")).toHaveAttribute("title", "in_progress / pending");
  });

  it("falls back to Unknown when both fields are absent", () => {
    render(<StatusBadge />);
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });
});
