import { useQuery } from "@tanstack/react-query";
import type { RepoCard } from "@/lib/bindings/RepoCard";
import { call } from "@/lib/ipc";

export const REPOS_QUERY_KEY = ["repos"] as const;

/**
 * The dashboard's data.
 *
 * Refetches only when it has nothing: after that, `run://updated` patches the
 * cache card by card.
 */
export function useRepos(enabled: boolean) {
  return useQuery({
    queryKey: REPOS_QUERY_KEY,
    queryFn: () => call<RepoCard[]>("repos_list"),
    enabled,
    staleTime: Infinity,
  });
}
