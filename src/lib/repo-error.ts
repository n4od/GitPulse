/**
 * How a poll failure is described to a person.
 *
 * The code comes from `AppError::code` on the Rust side and is a stable
 * discriminator rather than prose. What travels with it is a message written
 * for a log line; a title plus a hint is what someone looking at a red card
 * actually needs.
 */

export interface RepoErrorPresentation {
  title: string;
  hint: string;
}

const PRESENTATIONS: Record<string, RepoErrorPresentation> = {
  auth_required: {
    title: "Sign-in required",
    hint: "The session expired. Sign in again and polling resumes.",
  },
  auth_denied: {
    title: "Authorization denied",
    hint: "GitHub refused the authorization for this account.",
  },
  token_revoked: {
    title: "Access revoked",
    hint: "The token was revoked on GitHub. Sign in again to restore access.",
  },
  auth_expired: {
    title: "Sign-in expired",
    hint: "The device code expired before it was approved.",
  },
  keyring_unavailable: {
    title: "Keychain unavailable",
    hint: "The OS keychain could not be read, so this session is memory-only.",
  },
  forbidden: {
    title: "No access",
    hint: "This account cannot read the repository. It may have been made private.",
  },
  not_found: {
    title: "Repository not found",
    hint: "It was renamed or deleted, or the account lost access to it.",
  },
  rate_limited: {
    title: "Rate limited",
    hint: "GitHub's hourly quota is spent. Polling resumes at the reset time.",
  },
  secondary_rate_limited: {
    title: "Rate limited",
    hint: "GitHub asked GitPulse to slow down. It backs off and retries.",
  },
  network: {
    title: "Network problem",
    hint: "GitHub could not be reached. This usually clears on its own.",
  },
  db: {
    title: "Local database error",
    hint: "The cache could not be read or written.",
  },
  config: {
    title: "Configuration problem",
    hint: "Check the client id and the requested scopes.",
  },
  internal: {
    title: "Unexpected error",
    hint: "Something went wrong that GitPulse did not expect.",
  },
};

const FALLBACK: RepoErrorPresentation = {
  title: "Poll failed",
  hint: "GitPulse could not read this repository.",
};

export function describeRepoError(
  code: string | null | undefined,
): RepoErrorPresentation {
  if (!code) return FALLBACK;

  return PRESENTATIONS[code] ?? FALLBACK;
}
