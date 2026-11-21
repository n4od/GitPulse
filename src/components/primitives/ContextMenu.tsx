import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { cn } from "@/lib/cn";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: ReactNode;
  onSelect: () => void;
}

export interface ContextMenuProps {
  /** Viewport coordinates of the click that opened the menu. */
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
  label?: string;
}

/**
 * A right-click menu.
 *
 * Positioned `fixed` at the pointer rather than absolutely inside the card, so
 * a menu opened near the bottom of the grid is not clipped by the card's own
 * bounds. It closes on Escape, on a click outside, and on scroll — a menu that
 * stays put while its card scrolls away points at nothing.
 */
export function ContextMenu({
  x,
  y,
  items,
  onClose,
  label = "Repository actions",
}: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") onClose();
    }

    function onPointerDown(event: MouseEvent): void {
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
  }, [onClose]);

  // Nudge it back inside the window when opened near an edge.
  useEffect(() => {
    const element = ref.current;
    if (!element) return;

    const rect = element.getBoundingClientRect();
    const overflowX = rect.right - window.innerWidth;
    const overflowY = rect.bottom - window.innerHeight;

    if (overflowX > 0) element.style.left = `${Math.max(4, x - overflowX - 8)}px`;
    if (overflowY > 0) element.style.top = `${Math.max(4, y - overflowY - 8)}px`;

    element.querySelector("button")?.focus();
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
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="menuitem"
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
          {item.icon}
          {item.label}
        </button>
      ))}
    </div>,
    document.body,
  );
}
