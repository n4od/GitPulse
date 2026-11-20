import { Activity } from "lucide-react";
import { AuthPanel } from "@/components/auth/AuthPanel";
import { EmptyState } from "@/components/shell/EmptyState";
import { Button } from "@/components/primitives/Button";
import {
  cancelSignIn,
  openVerificationUri,
  retrySignIn,
  startSignIn,
} from "@/lib/auth-commands";
import { navigate } from "@/lib/router";
import { useAuthStore, useSession } from "@/stores/auth";

/**
 * Landing route. Signed out, it is the sign-in panel; signed in, it is a
 * placeholder until the repository dashboard lands in M6.
 */
export function Home() {
  const session = useSession();
  const state = useAuthStore((store) => store.state);

  if (!session) {
    return (
      <div className="grid min-h-full place-items-center px-6 py-10">
        <AuthPanel
          state={state}
          onSignIn={() => {
            void startSignIn();
          }}
          onCancel={() => {
            void cancelSignIn();
          }}
          onRetry={() => {
            void retrySignIn();
          }}
          onReopen={openVerificationUri}
        />
      </div>
    );
  }

  return (
    <EmptyState
      icon={<Activity className="size-5" aria-hidden="true" />}
      title={`Signed in as ${session.login}`}
      description="The shell, status vocabulary and primitives are in place. The repository dashboard arrives with the polling milestone."
      action={
        <Button
          variant="secondary"
          size="sm"
          onClick={() => {
            navigate("/kitchen-sink");
          }}
        >
          Open component gallery
        </Button>
      }
    />
  );
}
