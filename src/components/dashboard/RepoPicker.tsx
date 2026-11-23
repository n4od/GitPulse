import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Search } from "lucide-react";
import type { AvailableRepo } from "@/lib/bindings/AvailableRepo";
import { Badge } from "@/components/primitives/Badge";
import { Button } from "@/components/primitives/Button";
import { Card, CardBody } from "@/components/primitives/Card";
import { Skeleton } from "@/components/primitives/Skeleton";
import { call } from "@/lib/ipc";
import { toPulseError } from "@/lib/errors";
import { REPOS_QUERY_KEY } from "@/hooks/useRepos";
import { toast } from "@/stores/toasts";

/**
 * Picks which repositories to watch. Shown when nothing is tracked yet, and
 * reachable from the dashboard afterwards.
 */
export function RepoPicker({ onClose }: { onClose?: () => void }) {
  const queryClient = useQueryClient();
  const [query, setQuery] = useState("");

  const available = useQuery({
    queryKey: ["repos", "available"],
    queryFn: () => call<AvailableRepo[]>("repos_available"),
  });

  const track = useMutation({
    mutationFn: (fullName: string) => call<number>("repos_track", { fullName }),
    onSuccess: async () => {
      // Structural change: a new card exists, so the list is refetched rather
      // than patched.
      await queryClient.invalidateQueries({ queryKey: REPOS_QUERY_KEY });
    },
    onError: (cause: unknown) => {
      toast("error", "Could not track that repository", toPulseError(cause).message);
    },
  });

  const repositories = (available.data ?? []).filter((repo) =>
    repo.fullName.toLowerCase().includes(query.trim().toLowerCase()),
  );

  return (
    <Card className="mx-auto w-full max-w-2xl">
      <CardBody className="space-y-3">
        <div className="flex items-center gap-2">
          <h2 className="text-sm font-semibold text-ink">Choose repositories</h2>
          {onClose ? (
            <Button size="sm" variant="ghost" className="ml-auto" onClick={onClose}>
              Done
            </Button>
          ) : null}
        </div>

        <div className="relative">
          <Search
            className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-ink-muted"
            aria-hidden="true"
          />
          <input
            type="search"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
            }}
            placeholder="Search your repositories…"
            aria-label="Search your repositories"
            className="h-8 w-full rounded-control border border-line bg-raised pl-8 pr-3 text-[13px] text-ink placeholder:text-ink-muted"
          />
        </div>

        {available.isPending ? (
          <div className="space-y-2">
            {Array.from({ length: 5 }, (_, index) => (
              <Skeleton key={index} className="h-8 w-full" />
            ))}
          </div>
        ) : available.isError ? (
          <p className="text-xs text-bad">{toPulseError(available.error).message}</p>
        ) : repositories.length === 0 ? (
          <p className="text-xs text-ink-muted">No repositories matched.</p>
        ) : (
          <ul className="max-h-80 divide-y divide-line overflow-y-auto">
            {repositories.map((repo) => (
              <li key={repo.githubId} className="flex items-center gap-2 py-1.5">
                <span className="truncate text-[13px] text-ink">{repo.fullName}</span>
                {repo.isPrivate ? <Badge tone="muted">private</Badge> : null}

                <div className="ml-auto">
                  {repo.tracked ? (
                    <Badge tone="success">watching</Badge>
                  ) : (
                    <Button
                      size="sm"
                      variant="secondary"
                      loading={track.isPending && track.variables === repo.fullName}
                      leadingIcon={<Plus className="size-3.5" />}
                      onClick={() => {
                        track.mutate(repo.fullName);
                      }}
                    >
                      Watch
                    </Button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </CardBody>
    </Card>
  );
}
