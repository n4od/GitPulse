import { useEffect, useState } from "react";

/**
 * True when motion is allowed *right now*: the user has not asked for reduced
 * motion and the window is visible.
 *
 * Every looping animation in the app must be gated on this, otherwise a hidden
 * window keeps burning CPU — which would break the <1% idle budget.
 */
export function useMotionAllowed(): boolean {
  const [allowed, setAllowed] = useState(() => compute());

  useEffect(() => {
    const query =
      typeof window.matchMedia === "function"
        ? window.matchMedia("(prefers-reduced-motion: reduce)")
        : null;

    const update = () => setAllowed(!(query?.matches ?? false) && isVisible());

    query?.addEventListener("change", update);
    document.addEventListener("visibilitychange", update);
    update();

    return () => {
      query?.removeEventListener("change", update);
      document.removeEventListener("visibilitychange", update);
    };
  }, []);

  return allowed;
}

function compute(): boolean {
  const reduced =
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  return !reduced && isVisible();
}

function isVisible(): boolean {
  return document.visibilityState !== "hidden";
}
