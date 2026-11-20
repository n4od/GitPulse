import { Button } from "@/components/primitives/Button";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Toaster } from "@/components/primitives/Toast";
import { TitleBar } from "@/components/shell/TitleBar";
import { useAuthBridge } from "@/hooks/useAuthBridge";
import { signOut } from "@/lib/auth-commands";
import { navigate, useRoute } from "@/lib/router";
import { Home } from "@/pages/Home";
import { KitchenSink } from "@/pages/KitchenSink";
import { useSession } from "@/stores/auth";

/**
 * Application shell: header, routed content, global toast region.
 *
 * Routing is hash-based and dependency-free. The gallery route is compiled out
 * of production builds so the shipped bundle never carries dev-only screens.
 */
export default function App() {
  const route = useRoute();
  const session = useSession();

  // Connects the auth store to the backend exactly once, for the whole tree.
  useAuthBridge();

  return (
    <ErrorBoundary>
      <div className="flex h-full flex-col bg-surface text-ink">
        <TitleBar
          actions={
            <>
              {import.meta.env.DEV ? (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    navigate("/kitchen-sink");
                  }}
                >
                  Gallery
                </Button>
              ) : null}
              {session ? (
                <div className="flex items-center gap-2">
                  {session.avatarUrl ? (
                    <img
                      src={session.avatarUrl}
                      alt=""
                      width={20}
                      height={20}
                      className="size-5 rounded-full border border-line"
                    />
                  ) : null}
                  <span className="text-xs text-ink-soft">{session.login}</span>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      void signOut();
                    }}
                  >
                    Sign out
                  </Button>
                </div>
              ) : null}
            </>
          }
        />

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
