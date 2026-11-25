import { useQuery } from "@tanstack/react-query";
import { listAccounts } from "@/lib/auth-commands";

/**
 * The accounts this machine can switch between.
 *
 * Deliberately not gated on being signed in. Signing out removes one account
 * from the keychain and leaves the others, so a signed-out window is exactly
 * when this list matters most — gating it there is what made signing out of one
 * account look like signing out of the machine.
 */
export function useAccounts() {
  return useQuery({
    queryKey: ["accounts"],
    queryFn: listAccounts,
    // The list changes when an account is added or removed, both of which the
    // window already knows about, so there is nothing to poll for.
    staleTime: Infinity,
  });
}
