import type { ReactNode } from "react";
import { AlertTriangle, LogIn, KeyRound } from "lucide-react";
import type { AuthState } from "@/lib/bindings/AuthState";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody } from "@/components/primitives/Card";
import { Spinner } from "@/components/primitives/Spinner";
import { DeviceCodeCard } from "@/components/auth/DeviceCodeCard";

export interface AuthPanelProps {
  state: AuthState;
  /**
   * The accounts still signed in on this machine.
   *
   * Signing out removes one account and leaves the rest, so the signed-out
   * screen is exactly where these belong — offering only "sign in" made a
   * decision about one account look like a decision about the machine.
   */
  accounts?: readonly string[];
  onSignIn: () => void;
  onCancel: () => void;
  onRetry: () => void;
  onReopen: (uri: string) => void;
  onContinueAs?: (login: string) => void;
}

/**
 * Purely presentational: it renders whatever `AuthState` it is handed and
 * reports intent through callbacks. Nothing here touches IPC, which is what
 * makes every phase testable without mocking the backend.
 */
export function AuthPanel({
  state,
  accounts = [],
  onSignIn,
  onCancel,
  onRetry,
  onReopen,
  onContinueAs,
}: AuthPanelProps) {
  switch (state.phase) {
    case "requestingCode":
      return (
        <Frame
          icon={<Spinner className="size-5 text-accent" />}
          title="Contacting GitHub"
          description="Requesting a device code…"
        />
      );

    case "awaitingUser":
    case "polling":
      return (
        <DeviceCodeCard
          userCode={state.userCode}
          verificationUri={state.verificationUri}
          expiresAt={state.expiresAt}
          polling={state.phase === "polling"}
          onCancel={onCancel}
          onReopen={() => onReopen(state.verificationUri)}
        />
      );

    case "denied":
      return (
        <Frame
          icon={<AlertTriangle className="size-5 text-warn" />}
          title="Access denied"
          description="You declined the request on GitHub. Nothing was granted."
          action={
            <Button variant="primary" size="sm" onClick={onSignIn}>
              Try again
            </Button>
          }
        />
      );

    case "expired":
      return (
        <Frame
          icon={<AlertTriangle className="size-5 text-warn" />}
          title="The code expired"
          description="GitHub device codes are short-lived. Request a new one to continue."
          action={
            <Button variant="primary" size="sm" onClick={onSignIn}>
              Get a new code
            </Button>
          }
        />
      );

    case "failed":
      return (
        <Frame
          icon={<AlertTriangle className="size-5 text-bad" />}
          title="Sign-in failed"
          description={state.message}
          action={
            <>
              <Button variant="primary" size="sm" onClick={onRetry}>
                Try again
              </Button>
              {state.code === "config" ? (
                <p className="pt-2 text-[11px] leading-4 text-ink-muted">
                  Set <code className="font-mono">GITPULSE_GITHUB_CLIENT_ID</code> to the
                  client id of an OAuth App or GitHub App with device flow enabled, then
                  restart GitPulse.
                </p>
              ) : null}
            </>
          }
        />
      );

    case "signedOut":
      return (
        <Frame
          icon={<LogIn className="size-5 text-accent" />}
          title="Sign in to GitHub"
          description={
            accounts.length > 0
              ? "These accounts are still signed in on this machine. Pick one to carry on, or sign in with a different account."
              : "GitPulse reads your workflow runs directly from GitHub. Your token is stored in this machine's keychain, never sent anywhere else."
          }
          action={
            <div className="flex w-full max-w-[280px] flex-col items-stretch gap-2">
              {accounts.map((login) => (
                <Button
                  key={login}
                  variant="secondary"
                  onClick={() => {
                    onContinueAs?.(login);
                  }}
                >
                  Continue as {login}
                </Button>
              ))}
              <Button
                variant={accounts.length > 0 ? "ghost" : "primary"}
                onClick={onSignIn}
                leadingIcon={<KeyRound className="size-3.5" />}
              >
                {accounts.length > 0
                  ? "Sign in with a different account"
                  : "Sign in with GitHub"}
              </Button>
            </div>
          }
        />
      );

    case "authorized":
      // The signed-in UI is the dashboard's job, not this panel's.
      return null;

    default: {
      // Exhaustiveness guard: adding a phase to AuthState fails the build here.
      const unreachable: never = state;
      return <Frame title="Unknown state" description={JSON.stringify(unreachable)} />;
    }
  }
}

interface FrameProps {
  icon?: ReactNode;
  title: string;
  description: string;
  action?: ReactNode;
}

function Frame({ icon, title, description, action }: FrameProps) {
  return (
    <Card className="w-full max-w-md">
      <CardBody className="flex flex-col items-center gap-3 py-8 text-center">
        {icon ? (
          <div className="grid size-10 place-items-center rounded-card border border-line bg-raised">
            {icon}
          </div>
        ) : null}
        <h2 className="text-sm font-semibold text-ink">{title}</h2>
        <p className="max-w-sm text-xs leading-5 text-ink-soft">{description}</p>
        {action ? <div className="flex flex-col items-center pt-1">{action}</div> : null}
      </CardBody>
    </Card>
  );
}
