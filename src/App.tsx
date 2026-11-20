import { ErrorBoundary } from "@/components/ErrorBoundary";

/**
 * Application shell. M0 scope: prove the stack renders with the design tokens
 * applied. The auth gate, router and dashboard arrive in M1/M2.
 */
export default function App() {
  return (
    <ErrorBoundary>
      <div className="flex h-full flex-col bg-surface text-ink">
        <header className="flex h-12 shrink-0 items-center gap-3 border-b border-line px-4">
          <PulseMark />
          <span className="text-sm font-semibold tracking-tight">Pulse</span>
          <span className="ml-auto rounded-full border border-line px-2 py-0.5 font-mono text-[11px] text-ink-muted">
            M0 · Foundation
          </span>
        </header>

        <main className="grid flex-1 place-items-center p-8">
          <div className="max-w-sm text-center">
            <h1 className="text-xl font-semibold tracking-tight">Foundation is in place</h1>
            <p className="mt-2 text-ink-soft">
              Tauri 2, React 19, TypeScript and Tailwind 4 are wired up. Authentication
              lands in the next milestone.
            </p>
            <ul className="mt-6 space-y-2 text-left text-xs text-ink-muted">
              {[
                "Native window with strict CSP",
                "Rust core with typed error boundary",
                "Design tokens and dark-first palette",
                "CI on Windows, macOS and Linux",
              ].map((item) => (
                <li key={item} className="flex items-center gap-2">
                  <span aria-hidden="true" className="size-1.5 rounded-full bg-ok" />
                  {item}
                </li>
              ))}
            </ul>
          </div>
        </main>
      </div>
    </ErrorBoundary>
  );
}

function PulseMark() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      className="text-accent"
    >
      <path
        d="M2 13h4l2.5-7 4 14 3-9 2 2h4.5"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
