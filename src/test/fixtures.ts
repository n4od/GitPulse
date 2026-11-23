import type { Repo } from "@/lib/bindings/Repo";
import type { RepoCard } from "@/lib/bindings/RepoCard";
import type { WorkflowRun } from "@/lib/bindings/WorkflowRun";

/** Fixtures shared by tests. Not a test file itself, so it adds no test cases. */

export function makeRepo(overrides: Partial<Repo> = {}): Repo {
  return {
    id: 1,
    githubId: 100,
    owner: "acme",
    name: "api",
    fullName: "acme/api",
    isPrivate: false,
    defaultBranch: "main",
    avatarUrl: null,
    monitored: true,
    lastPolledAt: null,
    lastError: null,
    lastErrorCode: null,
    lastErrorAt: null,
    createdAt: "2026-11-20T10:00:00Z",
    updatedAt: "2026-11-20T10:00:00Z",
    ...overrides,
  };
}

export function makeRun(overrides: Partial<WorkflowRun> = {}): WorkflowRun {
  return {
    id: 1,
    repoId: 1,
    githubId: 500,
    workflowName: "CI",
    displayTitle: "Fix the thing",
    event: "push",
    branch: "main",
    headSha: "a1b2c3d4e5f6",
    status: "completed",
    conclusion: "success",
    runNumber: 7,
    runAttempt: 1,
    htmlUrl: "https://github.com/acme/api/actions/runs/500",
    actorLogin: "octocat",
    createdAt: "2026-11-20T10:00:00Z",
    runStartedAt: "2026-11-20T10:00:05Z",
    updatedAt: "2026-11-20T10:04:00Z",
    ...overrides,
  };
}

export function makeCard(overrides: Partial<RepoCard> = {}): RepoCard {
  const failingRuns = overrides.failingRuns ?? 0;

  return {
    repo: makeRepo(),
    recentRuns: [makeRun()],
    failingRuns,
    // A failing card always carries the run it badges. The real projection pairs
    // them, so a fixture that did not would let a test pass on a shape the
    // backend cannot produce.
    failingRun:
      failingRuns > 0 ? makeRun({ id: 2, githubId: 499, conclusion: "failure" }) : null,
    hasActiveRun: false,
    ...overrides,
  };
}
