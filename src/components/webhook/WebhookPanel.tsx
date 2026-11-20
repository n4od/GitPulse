import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Copy, KeyRound, RefreshCw } from "lucide-react";
import type { TransportKind } from "@/lib/bindings/TransportKind";
import type { WebhookConfig } from "@/lib/bindings/WebhookConfig";
import { Badge } from "@/components/primitives/Badge";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody, CardHeader } from "@/components/primitives/Card";
import { Spinner } from "@/components/primitives/Spinner";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { relativeTime } from "@/lib/repo-selectors";
import { toast } from "@/stores/toasts";

const CONFIG_KEY = ["webhook", "config"] as const;

const TRANSPORTS: Array<{ id: TransportKind; label: string; hint: string }> = [
  { id: "off", label: "Off", hint: "Polling only. Always correct, just slower." },
  {
    id: "relay",
    label: "Relay",
    hint: "A public endpoint forwards deliveries. Needs only a URL.",
  },
  {
    id: "local",
    label: "Local + tunnel",
    hint: "Binds 127.0.0.1; you run the tunnel that exposes it.",
  },
];

/**
 * Webhook setup.
 *
 * The signing secret is shown only when asked for: it is the one value that
 * exists solely to be copied into GitHub's form, and it has no business
 * appearing on a screen that happens to be open.
 */
export function WebhookPanel({ onClose }: { onClose: () => void }) {
  const queryClient = useQueryClient();
  const [kind, setKind] = useState<TransportKind>("off");
  const [relayUrl, setRelayUrl] = useState("");
  const [secret, setSecret] = useState<string | null>(null);
  const [publicUrl, setPublicUrl] = useState("");

  const config = useQuery({
    queryKey: CONFIG_KEY,
    queryFn: () => call<WebhookConfig>("webhook_config"),
    // Keeps the delivery counter roughly live while the panel is open.
    refetchInterval: 5_000,
  });

  const save = useMutation({
    mutationFn: (input: { kind: TransportKind; relayUrl: string | null }) =>
      call<WebhookConfig>("webhook_set_transport", {
        kind: input.kind,
        relayUrl: input.relayUrl,
        localPort: input.kind === "local" ? 0 : null,
      }),
    onSuccess: (next) => {
      queryClient.setQueryData(CONFIG_KEY, next);
      toast("success", "Webhook transport updated");
    },
    onError: (cause: unknown) => {
      toast("error", "Could not change the transport", toPulseError(cause).message);
    },
  });

  async function revealSecret(): Promise<void> {
    try {
      setSecret(await call<string>("webhook_signing_secret"));
    } catch (cause) {
      toast("error", "Could not read the signing secret", toPulseError(cause).message);
    }
  }

  async function rotateSecret(): Promise<void> {
    try {
      setSecret(await call<string>("webhook_rotate_secret"));
      toast(
        "info",
        "Secret rotated",
        "Update it in GitHub too, or deliveries will be rejected.",
      );
    } catch (cause) {
      toast("error", "Could not rotate the secret", toPulseError(cause).message);
    }
  }

  async function copy(value: string | null, label: string): Promise<void> {
    if (!value) return;

    try {
      await navigator.clipboard.writeText(value);
      toast("success", `${label} copied`);
    } catch {
      toast("error", `Could not copy the ${label.toLowerCase()}`);
    }
  }

  async function testPublicUrl(): Promise<void> {
    try {
      const reachable = await call<boolean>("webhook_test_public_url", {
        url: publicUrl,
      });
      toast(
        reachable ? "success" : "error",
        reachable ? "The tunnel reaches GitPulse" : "The tunnel did not answer",
        reachable ? undefined : "Check the URL and that the tunnel is running.",
      );
    } catch (cause) {
      toast("error", "Could not reach that URL", toPulseError(cause).message);
    }
  }

  const current = config.data;

  return (
    <div className="absolute inset-0 z-40 grid place-items-center bg-surface/80 p-6">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <span className="text-[13px] font-medium">Webhooks</span>
          {current ? (
            <Badge tone={current.running ? "success" : "muted"} className="ml-2">
              {current.running ? "running" : "stopped"}
            </Badge>
          ) : null}
          <Button size="sm" variant="ghost" className="ml-auto" onClick={onClose}>
            Close
          </Button>
        </CardHeader>

        <CardBody className="space-y-3">
          {config.isPending ? (
            <div className="flex items-center gap-2 text-xs text-ink-muted">
              <Spinner className="size-3.5" /> Loading…
            </div>
          ) : config.isError ? (
            <p className="text-xs text-bad">{toPulseError(config.error).message}</p>
          ) : (
            <>
              <p className="text-[11px] leading-4 text-ink-muted">
                GitHub cannot deliver to <code className="font-mono">127.0.0.1</code>, so
                a local listener needs a public relay or tunnel in front of it. All
                transports verify the same HMAC signature.
              </p>

              <div role="radiogroup" aria-label="Webhook transport" className="space-y-1">
                {TRANSPORTS.map((transport) => (
                  <label
                    key={transport.id}
                    className="flex cursor-pointer items-start gap-2 rounded-control border border-line bg-raised px-3 py-2"
                  >
                    <input
                      type="radio"
                      name="webhook-transport"
                      checked={kind === transport.id}
                      onChange={() => {
                        setKind(transport.id);
                      }}
                      className="mt-0.5"
                    />
                    <span>
                      <span className="block text-xs text-ink">{transport.label}</span>
                      <span className="block text-[11px] text-ink-muted">
                        {transport.hint}
                      </span>
                    </span>
                  </label>
                ))}
              </div>

              {kind === "relay" ? (
                <label className="block space-y-1">
                  <span className="text-[11px] text-ink-muted">Relay URL</span>
                  <input
                    value={relayUrl}
                    onChange={(event) => {
                      setRelayUrl(event.target.value);
                    }}
                    placeholder="https://smee.io/…"
                    className="h-8 w-full rounded-control border border-line bg-raised px-2 font-mono text-xs text-ink focus-visible:outline-none"
                  />
                </label>
              ) : null}

              <div className="flex justify-end">
                <Button
                  size="sm"
                  variant="primary"
                  loading={save.isPending}
                  onClick={() => {
                    save.mutate({ kind, relayUrl: relayUrl === "" ? null : relayUrl });
                  }}
                >
                  Save
                </Button>
              </div>

              {current?.localUrl ? (
                <div className="space-y-1 border-t border-line pt-3">
                  <span className="text-[11px] text-ink-muted">Local endpoint</span>
                  <div className="flex items-center gap-2">
                    <code className="selectable flex-1 truncate rounded-control border border-line bg-surface px-2 py-1 font-mono text-[11px]">
                      {current.localUrl}
                    </code>
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="Copy the local endpoint"
                      onClick={() => {
                        void copy(current.localUrl, "Endpoint");
                      }}
                    >
                      <Copy className="size-3.5" />
                    </Button>
                  </div>

                  <div className="flex items-center gap-2 pt-1">
                    <input
                      value={publicUrl}
                      onChange={(event) => {
                        setPublicUrl(event.target.value);
                      }}
                      placeholder="https://your-tunnel.example"
                      aria-label="Public tunnel URL"
                      className="h-7 flex-1 rounded-control border border-line bg-raised px-2 font-mono text-[11px] text-ink focus-visible:outline-none"
                    />
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => {
                        void testPublicUrl();
                      }}
                    >
                      Test
                    </Button>
                  </div>
                </div>
              ) : null}

              <div className="space-y-1 border-t border-line pt-3">
                <div className="flex items-center gap-2">
                  <span className="text-[11px] text-ink-muted">Signing secret</span>
                  <Button
                    size="sm"
                    variant="ghost"
                    leadingIcon={<KeyRound className="size-3.5" />}
                    onClick={() => {
                      void revealSecret();
                    }}
                  >
                    Reveal
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    leadingIcon={<RefreshCw className="size-3.5" />}
                    onClick={() => {
                      void rotateSecret();
                    }}
                  >
                    Rotate
                  </Button>
                </div>

                {secret ? (
                  <div className="flex items-center gap-2">
                    <code className="selectable flex-1 truncate rounded-control border border-line bg-surface px-2 py-1 font-mono text-[11px]">
                      {secret}
                    </code>
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label="Copy the signing secret"
                      onClick={() => {
                        void copy(secret, "Secret");
                      }}
                    >
                      <Copy className="size-3.5" />
                    </Button>
                  </div>
                ) : null}
              </div>

              <div className="flex items-center gap-3 border-t border-line pt-3 text-[11px] text-ink-muted">
                <span className="tabular-nums">
                  {current?.deliveries ?? 0} deliveries
                </span>
                <span>
                  last:{" "}
                  {current?.lastDeliveryAt
                    ? relativeTime(current.lastDeliveryAt)
                    : "never"}
                </span>
                {current?.lastError ? (
                  <span className="truncate text-bad" title={current.lastError}>
                    {current.lastError}
                  </span>
                ) : null}
              </div>
            </>
          )}
        </CardBody>
      </Card>
    </div>
  );
}
