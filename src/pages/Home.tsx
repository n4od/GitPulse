import { AuthPanel } from "@/components/auth/AuthPanel";
import { Dashboard } from "@/pages/Dashboard";
import {
  cancelSignIn,
  openVerificationUri,
  retrySignIn,
  startSignIn,
} from "@/lib/auth-commands";
import { useAuthStore, useSession } from "@/stores/auth";

/**
 * The landing route: sign-in while signed out, the dashboard once signed in.
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

  return <Dashboard />;
}
