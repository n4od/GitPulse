import { useRef, useState } from "react";
import { ChevronDown, LogOut, Plus } from "lucide-react";
import type { AuthSession } from "@/lib/bindings/AuthSession";
import { Avatar } from "@/components/primitives/Avatar";
import { ContextMenu, type ContextMenuItem } from "@/components/primitives/ContextMenu";
import { cn } from "@/lib/cn";

export interface AccountMenuProps {
  session: AuthSession;
  /** Every account signed in on this machine, lowercased. */
  accounts: readonly string[];
  onSwitch: (login: string) => void;
  onAddAccount: () => void;
  onSignOut: () => void;
}

/**
 * GitHub's public avatar endpoint for a login.
 *
 * A guess, and worth saying so: `auth_accounts` returns logins, not identities.
 * The avatar URL *is* stored per account — in that account's token metadata —
 * but reading it means a keychain read per row, and this endpoint answers the
 * same question without one. It needs no token and no API call.
 */
function avatarUrl(login: string): string {
  return `https://avatars.githubusercontent.com/${encodeURIComponent(login)}`;
}

/**
 * The account menu, hanging off the avatar in the header.
 *
 * This is where an account changes. It was in Settings, beside notifications and
 * polling, which made "which account am I on?" look like a behaviour preference
 * rather than an identity — and split the three account actions (switch, add,
 * sign out) across two places.
 */
export function AccountMenu({
  session,
  accounts,
  onSwitch,
  onAddAccount,
  onSignOut,
}: AccountMenuProps) {
  const trigger = useRef<HTMLButtonElement>(null);
  const [anchor, setAnchor] = useState<{ x: number; y: number } | null>(null);

  const active = session.login.toLowerCase();

  function open(): void {
    const rect = trigger.current?.getBoundingClientRect();
    if (!rect) {
      return;
    }

    // Left-aligned under the trigger; the menu's own edge-flip moves it back
    // inside the window when the header sits near the right edge.
    setAnchor({ x: rect.left, y: rect.bottom + 6 });
  }

  const items: ContextMenuItem[] = [
    ...accounts.map((login): ContextMenuItem => ({
      id: `account-${login}`,
      label: login,
      // The active account's own stored picture when we have it, rather than
      // the same one fetched twice.
      leading: (
        <Avatar
          login={login}
          src={
            login === active ? (session.avatarUrl ?? avatarUrl(login)) : avatarUrl(login)
          }
        />
      ),
      checked: login === active,
      onSelect: () => {
        // Selecting the active account is a no-op rather than a re-switch:
        // there is nothing to change.
        if (login !== active) {
          onSwitch(login);
        }
      },
    })),
    { id: "divider", separator: true },
    {
      id: "add-account",
      label: "Add another account...",
      leading: <Plus className="size-3.5" />,
      onSelect: onAddAccount,
    },
    {
      id: "sign-out",
      label: `Sign out of ${session.login}`,
      leading: <LogOut className="size-3.5" />,
      onSelect: onSignOut,
    },
  ];

  return (
    <>
      <button
        ref={trigger}
        type="button"
        aria-haspopup="menu"
        aria-expanded={anchor !== null}
        aria-label={`Account: ${session.login}`}
        onClick={() => {
          // The menu ignores clicks on this button, so this is the only toggle.
          if (anchor) {
            setAnchor(null);
          } else {
            open();
          }
        }}
        className={cn(
          "flex items-center gap-2 rounded-control px-1.5 py-1 text-xs text-ink-soft",
          "transition-colors hover:bg-raised hover:text-ink",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
        )}
      >
        <Avatar login={session.login} src={session.avatarUrl} />
        <span className="max-w-32 truncate">{session.login}</span>
        <ChevronDown className="size-3 text-ink-muted" aria-hidden="true" />
      </button>

      {anchor ? (
        <ContextMenu
          x={anchor.x}
          y={anchor.y}
          items={items}
          trigger={trigger}
          label={`Account menu for ${session.login}`}
          onClose={() => {
            setAnchor(null);
          }}
        />
      ) : null}
    </>
  );
}
