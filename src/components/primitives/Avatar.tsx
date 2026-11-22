import { useState } from "react";
import { cn } from "@/lib/cn";

export interface AvatarProps {
  /** GitHub's avatar URL, when one is known. */
  src?: string | null;
  /** The account, for the initial and nothing else. */
  login: string;
  className?: string;
}

/**
 * A circular account picture, or the account's initial.
 *
 * The initial is not decoration: an avatar URL can be absent or fail, and a
 * control — or a row — that collapses to nothing when its image 404s is worse
 * than one that shows a letter.
 *
 * `aria-hidden` throughout: every caller already names the account in text, and
 * announcing a picture as well is noise.
 */
export function Avatar({ src, login, className }: AvatarProps) {
  const [broken, setBroken] = useState(false);

  if (!src || broken) {
    return (
      <span
        aria-hidden="true"
        className={cn(
          "grid size-5 shrink-0 place-items-center rounded-full border border-line bg-raised",
          "text-[10px] font-semibold text-ink-soft",
          className,
        )}
      >
        {login.slice(0, 1).toUpperCase()}
      </span>
    );
  }

  return (
    <img
      src={src}
      alt=""
      width={20}
      height={20}
      onError={() => {
        setBroken(true);
      }}
      className={cn("size-5 shrink-0 rounded-full border border-line", className)}
    />
  );
}
