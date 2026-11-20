import { cn } from "@/lib/cn";
import { useMotionAllowed } from "@/hooks/useMotionAllowed";

interface SkeletonProps {
  className?: string;
}

/** Loading placeholder. Never animates when motion is disallowed or hidden. */
export function Skeleton({ className }: SkeletonProps) {
  const motion = useMotionAllowed();

  return (
    <div
      aria-hidden="true"
      className={cn("rounded bg-line/60", motion && "animate-pulse", className)}
    />
  );
}
