import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import type { WorkflowSummary } from "@/lib/bindings/WorkflowSummary";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody, CardHeader } from "@/components/primitives/Card";
import { Spinner } from "@/components/primitives/Spinner";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { toast } from "@/stores/toasts";

interface DispatchDialogProps {
  repoId: number;
  defaultRef: string;
  onClose: () => void;
}

/**
 * Triggers a `workflow_dispatch` run.
 *
 * Inputs are a JSON object rather than a generated form: reading each
 * workflow's `workflow_dispatch` schema means fetching and parsing the YAML,
 * which is a larger job than this dialog. The JSON box accepts exactly what
 * GitHub accepts, and is validated before sending.
 */
export function DispatchDialog({ repoId, defaultRef, onClose }: DispatchDialogProps) {
  const [workflowId, setWorkflowId] = useState<number | null>(null);
  const [gitRef, setGitRef] = useState(defaultRef);
  const [inputsJson, setInputsJson] = useState("{}");
  const [parseError, setParseError] = useState<string | null>(null);

  const workflows = useQuery({
    queryKey: ["workflows", repoId],
    queryFn: () => call<WorkflowSummary[]>("workflows_list", { repoId }),
  });

  const dispatch = useMutation({
    mutationFn: (input: {
      workflowId: number;
      gitRef: string;
      inputs: Record<string, string>;
    }) =>
      call("workflows_dispatch", {
        repoId,
        workflowId: input.workflowId,
        gitRef: input.gitRef,
        inputs: input.inputs,
      }),
    onSuccess: () => {
      toast("success", "Workflow dispatched", "GitHub will queue it shortly.");
      onClose();
    },
    onError: (cause: unknown) => {
      toast("error", "Dispatch failed", toPulseError(cause).message);
    },
  });

  function submit(): void {
    setParseError(null);

    if (workflowId === null) {
      setParseError("Choose a workflow first.");
      return;
    }

    let inputs: Record<string, string>;
    try {
      const parsed: unknown = JSON.parse(inputsJson === "" ? "{}" : inputsJson);

      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        throw new Error("Inputs must be a JSON object.");
      }

      inputs = Object.fromEntries(
        Object.entries(parsed as Record<string, unknown>).map(([key, value]) => [
          key,
          String(value),
        ]),
      );
    } catch (cause) {
      setParseError(
        cause instanceof Error ? cause.message : "Inputs are not valid JSON.",
      );
      return;
    }

    dispatch.mutate({ workflowId, gitRef, inputs });
  }

  return (
    <div className="absolute inset-0 z-40 grid place-items-center bg-surface/80 p-6">
      <Card className="w-full max-w-md">
        <CardHeader>
          <span className="text-[13px] font-medium">Dispatch a workflow</span>
          <Button size="sm" variant="ghost" className="ml-auto" onClick={onClose}>
            Cancel
          </Button>
        </CardHeader>

        <CardBody className="space-y-3">
          {workflows.isPending ? (
            <div className="flex items-center gap-2 text-xs text-ink-muted">
              <Spinner className="size-3.5" /> Loading workflows…
            </div>
          ) : workflows.isError ? (
            <p className="text-xs text-bad">{toPulseError(workflows.error).message}</p>
          ) : (
            <label className="block space-y-1">
              <span className="text-[11px] text-ink-muted">Workflow</span>
              <select
                value={workflowId ?? ""}
                onChange={(event) => {
                  setWorkflowId(
                    event.target.value === "" ? null : Number(event.target.value),
                  );
                }}
                className="h-8 w-full rounded-control border border-line bg-raised px-2 text-xs text-ink focus-visible:outline-none"
              >
                <option value="">Choose…</option>
                {(workflows.data ?? []).map((workflow) => (
                  <option key={workflow.id} value={workflow.id}>
                    {workflow.name} ({workflow.state})
                  </option>
                ))}
              </select>
            </label>
          )}

          <label className="block space-y-1">
            <span className="text-[11px] text-ink-muted">Branch or tag</span>
            <input
              value={gitRef}
              onChange={(event) => {
                setGitRef(event.target.value);
              }}
              className="h-8 w-full rounded-control border border-line bg-raised px-2 font-mono text-xs text-ink focus-visible:outline-none"
            />
          </label>

          <label className="block space-y-1">
            <span className="text-[11px] text-ink-muted">
              Inputs (JSON object, <code className="font-mono">{"{}"}</code> for none)
            </span>
            <textarea
              value={inputsJson}
              onChange={(event) => {
                setInputsJson(event.target.value);
              }}
              rows={4}
              spellCheck={false}
              className="w-full rounded-control border border-line bg-raised p-2 font-mono text-[11px] text-ink focus-visible:outline-none"
            />
          </label>

          {parseError ? <p className="text-[11px] text-bad">{parseError}</p> : null}

          <p className="text-[11px] leading-4 text-ink-muted">
            This runs on GitHub, against your repository — not locally.
          </p>

          <div className="flex justify-end gap-2">
            <Button
              size="sm"
              variant="primary"
              loading={dispatch.isPending}
              onClick={submit}
            >
              Dispatch
            </Button>
          </div>
        </CardBody>
      </Card>
    </div>
  );
}
