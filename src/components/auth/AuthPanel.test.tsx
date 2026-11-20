import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AuthPanel } from "@/components/auth/AuthPanel";
import type { AuthState } from "@/lib/bindings/AuthState";

function renderPanel(
  state: AuthState,
  overrides: Partial<Parameters<typeof AuthPanel>[0]> = {},
) {
  const handlers = {
    onSignIn: vi.fn(),
    onCancel: vi.fn(),
    onRetry: vi.fn(),
    onReopen: vi.fn(),
  };

  render(<AuthPanel state={state} {...handlers} {...overrides} />);

  return handlers;
}

describe("AuthPanel", () => {
  it("offers a single sign-in action when signed out", async () => {
    const handlers = renderPanel({ phase: "signedOut" });

    await userEvent.click(screen.getByRole("button", { name: /sign in with github/i }));

    expect(handlers.onSignIn).toHaveBeenCalledTimes(1);
  });

  it("shows the device code while awaiting the user", () => {
    renderPanel({
      phase: "awaitingUser",
      userCode: "ABCD-1234",
      verificationUri: "https://github.com/login/device",
      expiresAt: new Date(Date.now() + 600_000).toISOString(),
    });

    expect(screen.getByText("ABCD-1234")).toBeInTheDocument();
    expect(screen.getByText("https://github.com/login/device")).toBeInTheDocument();
  });

  it("shows polling feedback once the user has been to the browser", () => {
    renderPanel({
      phase: "polling",
      userCode: "ABCD-1234",
      verificationUri: "https://github.com/login/device",
      expiresAt: new Date(Date.now() + 600_000).toISOString(),
    });

    expect(screen.getByText(/waiting for you to approve/i)).toBeInTheDocument();
  });

  it("keeps the reopen link available while polling", () => {
    renderPanel({
      phase: "polling",
      userCode: "ABCD-1234",
      verificationUri: "https://github.com/login/device",
      expiresAt: new Date(Date.now() + 600_000).toISOString(),
    });

    expect(screen.getByRole("button", { name: /reopen github/i })).toBeInTheDocument();
  });

  it("lets the user cancel a flow in progress", async () => {
    const handlers = renderPanel({
      phase: "awaitingUser",
      userCode: "ABCD-1234",
      verificationUri: "https://github.com/login/device",
      expiresAt: new Date(Date.now() + 600_000).toISOString(),
    });

    await userEvent.click(screen.getByRole("button", { name: /cancel/i }));

    expect(handlers.onCancel).toHaveBeenCalledTimes(1);
  });

  it("explains a denial and offers a retry", async () => {
    const handlers = renderPanel({ phase: "denied" });

    expect(screen.getByRole("heading", { name: /access denied/i })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(handlers.onSignIn).toHaveBeenCalledTimes(1);
  });

  it("treats an expired code as recoverable", () => {
    renderPanel({ phase: "expired" });

    expect(screen.getByRole("heading", { name: /code expired/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /get a new code/i })).toBeInTheDocument();
  });

  it("surfaces the backend message on failure", () => {
    renderPanel({
      phase: "failed",
      code: "network",
      message: "network request failed",
    });

    expect(screen.getByText("network request failed")).toBeInTheDocument();
  });

  it("points at the missing configuration when that is the cause", () => {
    renderPanel({
      phase: "failed",
      code: "config",
      message: "no GitHub client id is configured",
    });

    expect(screen.getByText("PULSE_GITHUB_CLIENT_ID")).toBeInTheDocument();
  });

  it("renders nothing once authorized, leaving the dashboard in charge", () => {
    const { container } = render(
      <AuthPanel
        state={{
          phase: "authorized",
          session: {
            login: "octocat",
            avatarUrl: null,
            displayName: null,
            expiresAt: null,
            scopes: [],
            canRefresh: false,
          },
        }}
        onSignIn={vi.fn()}
        onCancel={vi.fn()}
        onRetry={vi.fn()}
        onReopen={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });
});
