import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AccountMenu } from "@/components/shell/AccountMenu";
import type { AuthSession } from "@/lib/bindings/AuthSession";

function session(overrides: Partial<AuthSession> = {}): AuthSession {
  return {
    login: "n4od",
    avatarUrl: null,
    displayName: null,
    expiresAt: null,
    scopes: [],
    canRefresh: true,
    ...overrides,
  };
}

function setup(overrides: Partial<Parameters<typeof AccountMenu>[0]> = {}) {
  const handlers = {
    onSwitch: vi.fn(),
    onAddAccount: vi.fn(),
    onSignOut: vi.fn(),
  };

  render(
    <AccountMenu
      session={session()}
      accounts={["birukboss", "n4od"]}
      {...handlers}
      {...overrides}
    />,
  );

  return handlers;
}

describe("AccountMenu", () => {
  it("shows the active account on the trigger", () => {
    setup();

    expect(screen.getByRole("button", { name: /account: n4od/i })).toBeInTheDocument();
  });

  it("falls back to an initial when there is no avatar", () => {
    setup();

    // A trigger that disappears with the avatar is a control nobody can find.
    expect(screen.getByText("N")).toBeInTheDocument();
  });

  it("is closed until the trigger is pressed", async () => {
    const user = userEvent.setup();
    setup();

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));

    expect(screen.getByRole("menu")).toBeInTheDocument();
  });

  it("marks the active account and offers the others", async () => {
    const user = userEvent.setup();
    setup();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));
    const menu = screen.getByRole("menu");

    expect(
      within(menu).getByRole("menuitemcheckbox", { name: "n4od", checked: true }),
    ).toBeInTheDocument();
    expect(
      within(menu).getByRole("menuitemcheckbox", { name: "birukboss", checked: false }),
    ).toBeInTheDocument();
  });

  it("switches to another account", async () => {
    const user = userEvent.setup();
    const handlers = setup();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));
    await user.click(
      within(screen.getByRole("menu")).getByRole("menuitemcheckbox", {
        name: "birukboss",
      }),
    );

    expect(handlers.onSwitch).toHaveBeenCalledWith("birukboss");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("does not re-switch when the active account is chosen", async () => {
    const user = userEvent.setup();
    const handlers = setup();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));
    await user.click(
      within(screen.getByRole("menu")).getByRole("menuitemcheckbox", { name: "n4od" }),
    );

    expect(handlers.onSwitch).not.toHaveBeenCalled();
  });

  it("offers adding another account and signing out", async () => {
    const user = userEvent.setup();
    const handlers = setup();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));
    const menu = screen.getByRole("menu");

    await user.click(
      within(menu).getByRole("menuitem", { name: /add another account/i }),
    );
    expect(handlers.onAddAccount).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));
    await user.click(
      within(screen.getByRole("menu")).getByRole("menuitem", {
        name: /sign out of n4od/i,
      }),
    );
    expect(handlers.onSignOut).toHaveBeenCalledTimes(1);
  });

  it("toggles shut when the trigger is pressed again", async () => {
    const user = userEvent.setup();
    setup();

    const trigger = screen.getByRole("button", { name: /account: n4od/i });
    await user.click(trigger);
    expect(screen.getByRole("menu")).toBeInTheDocument();

    // The menu must ignore a click on its own trigger, or the outside-click
    // handler closes it and the trigger's click reopens it.
    await user.click(trigger);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("closes on Escape", async () => {
    const user = userEvent.setup();
    setup();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));
    expect(screen.getByRole("menu")).toBeInTheDocument();

    await user.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("moves between rows with the arrow keys", async () => {
    const user = userEvent.setup();
    setup();

    await user.click(screen.getByRole("button", { name: /account: n4od/i }));

    // The first row takes focus on open; one ArrowDown lands on the next account.
    await user.keyboard("{ArrowDown}");

    expect(
      within(screen.getByRole("menu")).getByRole("menuitemcheckbox", {
        name: "n4od",
      }),
    ).toHaveFocus();
  });
});
