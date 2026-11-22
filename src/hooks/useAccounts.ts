import { useQuery } from "@tanstack/react-query";
import { listAccounts } from "@/lib/auth-commands";

/**
 * The accounts this machine can switch between.
 *
 * Only meaningful while signed in — the list lives in the keychain, and a
 * signed-out window has none.
 */
export function useAccounts(enabled: boolean) {
  return useQuery({
    queryKey: ["accounts"],
    queryFn: listAccounts,
    enabled,
    // The list changes when an account is added, which is a sign-in the window
    // already knows about, so there is nothing to poll for.
    staleTime: Infinity,
  });
}
