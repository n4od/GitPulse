import type { ReactNode } from "react";
import { Copy, GitBranch, Lock, Play, RotateCcw, XCircle } from "lucide-react";
import { Badge } from "@/components/primitives/Badge";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody, CardHeader } from "@/components/primitives/Card";
import { Skeleton } from "@/components/primitives/Skeleton";
import { Spinner } from "@/components/primitives/Spinner";
import { Tooltip } from "@/components/primitives/Tooltip";
import { EmptyState } from "@/components/shell/EmptyState";
import { StatusBadge } from "@/components/status/StatusBadge";
import { StatusPulse } from "@/components/status/StatusPulse";
import { toast } from "@/stores/toasts";
import { navigate } from "@/lib/router";
import type { StatusTone } from "@/lib/status";

const TONES: StatusTone[] = ["neutral", "running", "success", "failure", "warn", "muted"];

const STATUS_ROWS: Array<{ status: string; conclusion: string | null }> = [
  { status: "queued", conclusion: null },
  { status: "waiting", conclusion: null },
  { status: "pending", conclusion: null },
  { status: "in_progress", conclusion: null },
  { status: "completed", conclusion: "success" },
  { status: "completed", conclusion: "failure" },
  { status: "completed", conclusion: "cancelled" },
  { status: "completed", conclusion: "timed_out" },
  { status: "completed", conclusion: "action_required" },
  { status: "completed", conclusion: "neutral" },
  { status: "completed", conclusion: "skipped" },
  { status: "completed", conclusion: "stale" },
  { status: "completed", conclusion: "startup_failure" },
  { status: "teleported", conclusion: null },
];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="space-y-3">
      <h2 className="text-xs font-semibold uppercase tracking-wider text-ink-muted">
        {title}
      </h2>
      {children}
    </section>
  );
}

/**
 * Dev-only gallery. Every primitive is rendered in every state so visual
 * regressions and missing keyboard states are caught by eye before M6.
 */
export function KitchenSink() {
  return (
    <div className="mx-auto max-w-5xl space-y-10 px-6 py-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">Component gallery</h1>
          <p className="mt-1 text-xs text-ink-soft">
            Every primitive in every state. Not shipped in production builds.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={() => navigate("/")}>
          Back to app
        </Button>
      </div>

      <Section title="Buttons">
        <div className="flex flex-wrap items-center gap-2">
          <Button variant="primary">Primary</Button>
          <Button variant="secondary">Secondary</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="danger">Danger</Button>
          <Button variant="primary" loading>
            Loading
          </Button>
          <Button variant="secondary" disabled>
            Disabled
          </Button>
          <Button
            variant="secondary"
            size="sm"
            leadingIcon={<RotateCcw className="size-3.5" />}
          >
            Re-run
          </Button>
          <Button variant="primary" size="sm" leadingIcon={<Play className="size-3.5" />}>
            Dispatch
          </Button>
          <Button variant="ghost" size="sm" leadingIcon={<Copy className="size-3.5" />}>
            Copy
          </Button>
          <Button
            variant="ghost"
            size="sm"
            leadingIcon={<XCircle className="size-3.5" />}
          >
            Cancel
          </Button>
        </div>
      </Section>

      <Section title="Badges and status">
        <div className="flex flex-wrap items-center gap-2">
          {TONES.map((tone) => (
            <Badge
              key={tone}
              tone={tone}
              indicator={<StatusPulse tone={tone} pulse={tone === "running"} />}
            >
              {tone}
            </Badge>
          ))}
          <Badge tone="accent">accent</Badge>
        </div>
        <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
          {STATUS_ROWS.map((row) => (
            <div
              key={`${row.status}-${row.conclusion ?? "none"}`}
              className="flex items-center justify-between gap-3 rounded-control border border-line bg-raised px-3 py-2"
            >
              <code className="selectable font-mono text-[11px] text-ink-muted">
                {row.status}/{row.conclusion ?? "—"}
              </code>
              <StatusBadge status={row.status} conclusion={row.conclusion} />
            </div>
          ))}
        </div>
      </Section>

      <Section title="Cards">
        <div className="grid gap-3 sm:grid-cols-2">
          <Card>
            <CardHeader>
              <Lock className="size-3.5 text-ink-muted" aria-hidden="true" />
              <span className="text-[13px] font-medium">acme/api</span>
              <StatusBadge status="completed" conclusion="failure" className="ml-auto" />
            </CardHeader>
            <CardBody className="space-y-2">
              <div className="flex items-center gap-2 text-xs text-ink-soft">
                <GitBranch className="size-3.5" aria-hidden="true" />
                <span>main</span>
                <span className="font-mono text-[11px] text-ink-muted">a1b2c3d</span>
                <span className="ml-auto">2m ago</span>
              </div>
              <div className="flex gap-2">
                <Button
                  size="sm"
                  variant="secondary"
                  leadingIcon={<RotateCcw className="size-3.5" />}
                >
                  Re-run failed
                </Button>
                <Tooltip label="Copy run URL">
                  <Button size="sm" variant="ghost" aria-label="Copy run URL">
                    <Copy className="size-3.5" />
                  </Button>
                </Tooltip>
              </div>
            </CardBody>
          </Card>

          <Card>
            <CardHeader>
              <Skeleton className="h-3 w-24" />
              <Skeleton className="ml-auto h-4 w-16 rounded-full" />
            </CardHeader>
            <CardBody className="space-y-2">
              <Skeleton className="h-3 w-40" />
              <Skeleton className="h-3 w-28" />
              <div className="flex gap-2 pt-1">
                <Skeleton className="h-7 w-24 rounded-control" />
                <Skeleton className="h-7 w-16 rounded-control" />
              </div>
            </CardBody>
          </Card>
        </div>
      </Section>

      <Section title="Feedback">
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            onClick={() => toast("info", "Polling paused", "Resumes in 5 minutes.")}
          >
            Info toast
          </Button>
          <Button
            size="sm"
            onClick={() => toast("success", "Re-run queued", "acme/api · deploy.yml")}
          >
            Success toast
          </Button>
          <Button
            size="sm"
            onClick={() =>
              toast(
                "error",
                "Dispatch failed",
                "Workflow has no workflow_dispatch trigger.",
              )
            }
          >
            Error toast
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => toast("info", "Spinner inline", "See the button.")}
          >
            <Spinner className="size-3.5" /> Inline spinner
          </Button>
        </div>
      </Section>

      <Section title="Empty state">
        <Card>
          <EmptyState
            title="No repositories yet"
            description="Once authentication lands you can pick which repositories GitPulse should watch."
            action={
              <Button size="sm" variant="primary" disabled>
                Sign in with GitHub
              </Button>
            }
          />
        </Card>
      </Section>

      <Section title="Typography">
        <div className="space-y-2">
          <p className="text-2xl font-semibold tracking-tight">Heading 24</p>
          <p className="text-lg font-semibold tracking-tight">Heading 18</p>
          <p className="text-[13px]">
            Body 13 — the workhorse size for dense desktop UI.
          </p>
          <p className="text-xs text-ink-soft">Secondary 12 — metadata, timestamps.</p>
          <p className="font-mono text-[11px] text-ink-muted">mono 11 · 0f3a9c2</p>
        </div>
      </Section>
    </div>
  );
}
