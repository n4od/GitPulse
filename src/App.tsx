import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Toaster } from "@/components/primitives/Toast";
import { TitleBar } from "@/components/shell/TitleBar";
import { Home } from "@/pages/Home";
import { KitchenSink } from "@/pages/KitchenSink";
import { useRoute } from "@/lib/router";

/**
 * Application shell: header, routed content, global toast region.
 *
 * Routing is hash-based and dependency-free. The gallery route is compiled out
 * of production builds so the shipped bundle never carries dev-only screens.
 */
export default function App() {
  const route = useRoute();

  return (
    <ErrorBoundary>
      <div className="flex h-full flex-col bg-surface text-ink">
        <TitleBar />
        <main className="min-h-0 flex-1 overflow-y-auto">
          {import.meta.env.DEV && route.name === "kitchen-sink" ? (
            <KitchenSink />
          ) : (
            <Home />
          )}
        </main>
        <Toaster />
      </div>
    </ErrorBoundary>
  );
}
