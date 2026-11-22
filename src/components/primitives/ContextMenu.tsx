import { useEffect, useRef, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";
import { Check } from "lucide-react";
import { cn } from "@/lib/cn";

/**
 * One row in a menu.
 *
 * A separator has no label and no action, so it is a separate shape rather than
 * an item with an empty `onSelect`: there is no such thing as a separator that
 * does something when clicked, and the type should not admit one.
 */
export type ContextMenuItem =
  | {
      id: string;
      label: string;
      /** The leading visual: an icon, or an `<Avatar>` for an account row. */
      leading?: ReactNode;
      /** Renders a tick at the trailing edge and marks the row as a choice. */
      checked?: boolean;
      separator?: false;
      onSelect: () => void;
    }
  | { id: string; separator: true };

export interface ContextMenuProps {
  /** Viewport coordinates of the click that opened the menu. */
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
  label?: string;
  /**
   * The control that opened the menu.
   *
   * A click on it is a toggle, not a click outside — without this, pressing the
   * trigger while its own menu is open closes it (the outside-click handler) and
   * then reopens it (the trigger's click), so the menu never toggles off.
   */
  trigger?: RefObject<HTMLElement | null>;
}

/**
 * A menu, positioned at a point.
 *
 * Rendered `fixed` rather than absolutely inside its trigger, so a menu opened
 * near the bottom of a virtualised list is not clipped by that row's bounds. It
 * closes on Escape, on a click outside, and on scroll — a menu that stays put
 * while what it belongs to scrolls away points at nothing.
 */
export function ContextMenu({
  x,
  y,
  items,
  onClose,
  label = "Actions",
  trigger,
}: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Focus goes back to whatever had it, which for a menu opened from a button
    // is that button. Restoring it here rather than in the caller means every
    // menu gets it, including the repository cards'.
    const previous = document.activeElement as HTMLElement | null;
    return () => {
      previous?.focus?.();
    };
  }, []);

  useEffect(() => {
    function rows(): HTMLButtonElement[] {
      return Array.from(
        ref.current?.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]') ?? [],
      );
    }

    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }

      const movement: Record<string, (index: number, count: number) => number> = {
        ArrowDown: (index, count) => (index < 0 ? 0 : (index + 1) % count),
        ArrowUp: (index, count) => (index < 0 ? count - 1 : (index - 1 + count) % count),
        Home: () => 0,
        End: (_index, count) => count - 1,
      };

      const move = movement[event.key];
      if (!move) return;

      const buttons = rows();
      if (buttons.length === 0) return;

      event.preventDefault();
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      buttons[move(index, buttons.length)]?.focus();
    }

    function onPointerDown(event: MouseEvent): void {
      if (trigger?.current?.contains(event.target as Node)) return;
      if (ref.current && !ref.current.contains(event.target as Node)) onClose();
    }

    function onScroll(): void {
      onClose();
    }

    window.addEventListener("keydown", onKeyDown);
    // Capture, so a click on another card closes this menu before that card
    // opens its own.
    window.addEventListener("mousedown", onPointerDown, true);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("mousedown", onPointerDown, true);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
    };
  }, [onClose, trigger]);

  // Nudge it back inside the window when opened near an edge.
  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    const rect = element.getBoundingClientRect();
    const overflowX = rect.right - window.innerWidth;
    const overflowY = rect.bottom - window.innerHeight;

    if (overflowX > 0) element.style.left = `${Math.max(4, x - overflowX - 8)}px`;
    if (overflowY > 0) element.style.top = `${Math.max(4, y - overflowY - 8)}px`;

    element.querySelector<HTMLButtonElement>('[role^="menuitem"]')?.focus();
  }, [x, y]);

  // Rendered into `document.body` rather than in place.
  //
  // The dashboard's virtualised rows are positioned with
  // `transform: translateY(...)`, and a transformed ancestor makes
  // `position: fixed` resolve against *that ancestor* instead of the viewport.
  // Left in place, the menu lands offset by the row's transform — an offset that
  // grows the further down the list you are.
  return createPortal(
    <div
      ref={ref}
      role="menu"
      aria-label={label}
      className="fixed z-50 min-w-52 rounded-card border border-line-strong bg-overlay p-1 shadow-2xl"
      style={{ left: x, top: y }}
      onContextMenu={(event) => {
        event.preventDefault();
      }}
    >
      {items.map((item) => {
        if (item.separator) {
          return (
            <div
              key={item.id}
              role="separator"
              className="my-1 h-px bg-line"
              aria-hidden="true"
            />
          );
        }

        // A row whose state matters is a checkbox; a plain row is a menuitem.
        // Told apart by the type, so a checked row cannot be announced as an
        // action.
        const checkable = item.checked !== undefined;

        return (
          <button
            key={item.id}
            type="button"
            role={checkable ? "menuitemcheckbox" : "menuitem"}
            aria-checked={checkable ? item.checked : undefined}
            onClick={() => {
              onClose();
              item.onSelect();
            }}
            className={cn(
              "flex w-full items-center gap-2 rounded-control px-2.5 py-1.5 text-left text-[12px]",
              "text-ink-soft transition-colors hover:bg-raised hover:text-ink",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
            )}
          >
            {/* A fixed slot, so labels line up whether the row leads with a
                16px icon, a 20px avatar, or nothing. */}
            {item.leading ? (
              <span className="flex w-5 shrink-0 items-center justify-center">
                {item.leading}
              </span>
            ) : null}
            <span className="truncate">{item.label}</span>
            {/* The tick trails rather than leads, so a row can show both the
                account's picture and which account is active. */}
            {item.checked ? <Check className="ml-auto size-3.5 shrink-0" /> : null}
          </button>
        );
      })}
    </div>,
    document.body,
  );
}
