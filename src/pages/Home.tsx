import { Activity } from "lucide-react";
import { EmptyState } from "@/components/shell/EmptyState";
import { Button } from "@/components/primitives/Button";
import { navigate } from "@/lib/router";

/**
 * Landing route. M1 scope: prove the shell and design system render.
 * The repository dashboard replaces this in M6.
 */
export function Home() {
  return (
    <EmptyState
      icon={<Activity className="size-5" aria-hidden="true" />}
      title="Design system ready"
      description="The shell, status vocabulary and primitives are in place. The repository dashboard arrives with the polling milestone."
      action={
        <Button variant="secondary" size="sm" onClick={() => navigate("/kitchen-sink")}>
          Open component gallery
        </Button>
      }
    />
  );
}
