import { useEffect, useState } from "react";

export type Route = { name: "home" } | { name: "kitchen-sink" } | { name: "bench" };

function parse(hash: string): Route {
  const path = hash.replace(/^#/, "").replace(/^\/+/, "").replace(/\/+$/, "");

  if (path === "kitchen-sink") return { name: "kitchen-sink" };
  if (path === "bench") return { name: "bench" };

  return { name: "home" };
}

/** Hash-based routing: no dependency, works in a WebView, survives reload. */
export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parse(window.location.hash));

  useEffect(() => {
    const onChange = () => setRoute(parse(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);

  return route;
}

export function navigate(path: string): void {
  window.location.hash = path;
}
