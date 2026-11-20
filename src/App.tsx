import { useState } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { Button } from "@/components/primitives/Button";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Toaster } from "@/components/primitives/Toast";
import { RateLimitMeter } from "@/components/status/RateLimitMeter";
import { TitleBar } from "@/components/shell/TitleBar";
import { useAuthBridge } from "@/hooks/useAuthBridge";
import { useLiveUpdates } from "@/hooks/useLiveUpdates";
import { signOut } from "@/lib/auth-commands";
import { createQueryClient } from "@/lib/queryClient";
import { navigate, useRoute } from "@/lib/router";
import { Home } from "@/pages/Home";
import { KitchenSink } from "@/pages/KitchenSink";
import { Bench } from "@/pages/Bench";
import { useSession } from "@/stores/auth";

/**
 * Application shell: header, routed content, global toast region.
 *
 * The query client is created once per mount and shared through context, so
 * every hook in the tree talks to the same cache.
 */
export default function App() {
  const [queryClient] = useState(createQueryClient);

  return (
    <QueryClientProvider client={queryClient}>
      <ErrorBoundary>
        <Shell />
      </ErrorBoundary>
    </QueryClientProvider>
  );
}

function Shell() {
  const route = useRoute();
  const session = useSession();

  // Each connects to the backend exactly once, for the whole tree.
  useAuthBridge();
  useLiveUpdates();

  return (
    <div className="flex h-full flex-col bg-surface text-ink">
      <TitleBar
        actions={
          <>
            <RateLimitMeter />

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

      <main className="min-h-0 flex-1 overflow-hidden">
        {import.meta.env.DEV && route.name === "kitchen-sink" ? (
          <KitchenSink />
        ) : import.meta.env.DEV && route.name === "bench" ? (
          <Bench />
        ) : (
          <Home />
        )}
      </main>

      <Toaster />
    </div>
  );
}
