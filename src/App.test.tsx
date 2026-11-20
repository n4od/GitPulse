import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "@/App";

describe("App shell", () => {
  it("renders the product shell", () => {
    render(<App />);
    expect(screen.getByRole("heading", { name: /foundation is in place/i })).toBeInTheDocument();
    expect(screen.getByText("Pulse")).toBeInTheDocument();
  });
});
