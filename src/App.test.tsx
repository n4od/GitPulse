import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "@/App";

describe("App shell", () => {
  it("renders the title bar and the landing route", () => {
    render(<App />);
    expect(screen.getByText("Pulse")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: /design system ready/i }),
    ).toBeInTheDocument();
  });

  it("keeps the dev gallery out of the default route", () => {
    render(<App />);
    expect(
      screen.queryByRole("heading", { name: /component gallery/i }),
    ).not.toBeInTheDocument();
  });
});
