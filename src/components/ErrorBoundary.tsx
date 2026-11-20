import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

/**
 * Last line of defence: a render error in one panel must never blank the window.
 * Errors are surfaced in-place with a reload action rather than swallowed.
 */
export class ErrorBoundary extends Component<Props, State> {
  override state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  override componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("Unhandled render error", error, info.componentStack);
  }

  override render(): ReactNode {
    const { error } = this.state;
    if (!error) return this.props.children;

    return (
      <div className="grid h-full place-items-center p-8">
        <div className="w-full max-w-md rounded-card border border-line bg-raised p-6">
          <h1 className="text-lg font-semibold text-ink">Something broke</h1>
          <p className="mt-2 text-ink-soft">
            The interface hit an unexpected error. Your data is unaffected.
          </p>
          <pre className="selectable mt-4 max-h-40 overflow-auto rounded-control border border-line bg-surface p-3 font-mono text-xs text-ink-muted">
            {error.message}
          </pre>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="mt-4 rounded-control bg-accent px-3 py-2 font-medium text-surface"
          >
            Reload window
          </button>
        </div>
      </div>
    );
  }
}
