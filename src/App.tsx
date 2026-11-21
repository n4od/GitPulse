import { useEffect, useState } from "react";
import { QueryClientProvider, useQueryClient } from "@tanstack/react-query";
import { Settings } from "lucide-react";
import { Button } from "@/components/primitives/Button";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Toaster } from "@/components/primitives/Toast";
import { CommandPalette } from "@/components/shell/CommandPalette";
import { SettingsPanel } from "@/components/settings/SettingsPanel";
import { RateLimitMeter } from "@/components/status/RateLimitMeter";
import { StatusBar } from "@/components/shell/StatusBar";
import { TitleBar } from "@/components/shell/TitleBar";
import { WebhookPanel } from "@/components/webhook/WebhookPanel";
import { useAuthBridge } from "@/hooks/useAuthBridge";
import { useLiveUpdates } from "@/hooks/useLiveUpdates";
import { useRepos } from "@/hooks/useRepos";
import { signOut } from "@/lib/auth-commands";
import { call } from "@/lib/ipc";
import { createQueryClient } from "@/lib/queryClient";
import { navigate, useRoute } from "@/lib/router";
import { toast } from "@/stores/toasts";
import { Bench } from "@/pages/Bench";
import { Home } from "@/pages/Home";
import { KitchenSink } from "@/pages/KitchenSink";
import { useSession } from "@/stores/auth";
import { useUiStore } from "@/stores/ui";

type Overlay = "palette" | "settings" | "webhooks" | null;

/**
 * Application shell: header, routed content, global toast region, and the
 * keyboard-driven overlays.
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
  const queryClient = useQueryClient();
  const inspect = useUiStore((store) => store.inspect);

  const [overlay, setOverlay] = useState<Overlay>(null);

  // Both bridges connect to the backend exactly once, for the whole tree.
  useAuthBridge();
  useLiveUpdates();

  const repos = useRepos(Boolean(session));
  const cards = repos.data ?? [];

  // Ctrl/Cmd+K opens the palette; Escape closes whatever is open. Registered
  // here, at the root, so it works regardless of what has focus.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      const openPalette =
        (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k";

      if (openPalette) {
        event.preventDefault();
        setOverlay((current) => (current === "palette" ? null : "palette"));
        return;
      }

      if (event.key === "Escape") {
        setOverlay(null);
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  async function refreshAll(): Promise<void> {
    const ids = cards.map((card) => card.repo.id);

    await Promise.all(ids.map((repoId) => call("repos_refresh", { repoId })));
    toast("info", `Queued ${ids.length} ${ids.length === 1 ? "refresh" : "refreshes"}`);
  }

  function runPaletteAction(id: string): void {
    switch (id) {
      case "settings":
        setOverlay("settings");
        break;
      case "webhooks":
        setOverlay("webhooks");
        break;
      case "refresh-all":
        void refreshAll();
        break;
      case "sign-out":
        void signOut();
        queryClient.removeQueries({ queryKey: ["repos"] });
        break;
      default:
        break;
    }
  }

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
              <>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label="Settings"
                  title="Settings (Ctrl+K for the command palette)"
                  onClick={() => {
                    setOverlay("settings");
                  }}
                >
                  <Settings className="size-3.5" />
                </Button>

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
                </div>
              </>
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

      {session ? <StatusBar /> : null}

      <Toaster />

      {overlay === "settings" ? (
        <SettingsPanel
          onClose={() => {
            setOverlay(null);
          }}
          onOpenWebhooks={() => {
            setOverlay("webhooks");
          }}
        />
      ) : null}

      {overlay === "webhooks" ? (
        <WebhookPanel
          onClose={() => {
            setOverlay(null);
          }}
        />
      ) : null}

      {overlay === "palette" ? (
        <CommandPalette
          cards={cards}
          onClose={() => {
            setOverlay(null);
          }}
          onOpenRepo={inspect}
          onAction={runPaletteAction}
        />
      ) : null}
    </div>
  );
}
