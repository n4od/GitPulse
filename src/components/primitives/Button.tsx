import type { ComponentPropsWithRef, ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Spinner } from "@/components/primitives/Spinner";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

const VARIANTS: Record<Variant, string> = {
  primary: "bg-accent text-surface hover:bg-accent/90 active:bg-accent/80",
  secondary:
    "border border-line-strong bg-raised text-ink hover:border-line-strong hover:bg-overlay active:bg-raised",
  ghost: "text-ink-soft hover:bg-raised hover:text-ink active:bg-overlay",
  danger: "bg-bad text-surface hover:bg-bad/90 active:bg-bad/80",
};

const SIZES: Record<Size, string> = {
  sm: "h-7 gap-1.5 px-2.5 text-xs",
  md: "h-8 gap-2 px-3 text-[13px]",
};

export interface ButtonProps extends ComponentPropsWithRef<"button"> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  leadingIcon?: ReactNode;
}

export function Button({
  variant = "secondary",
  size = "md",
  loading = false,
  leadingIcon,
  className,
  children,
  disabled,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      disabled={disabled ?? loading}
      aria-busy={loading ? true : undefined}
      className={cn(
        "inline-flex select-none items-center justify-center rounded-control font-medium",
        "transition-colors duration-150",
        "disabled:pointer-events-none disabled:opacity-50",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface",
        VARIANTS[variant],
        SIZES[size],
        className,
      )}
      {...rest}
    >
      {loading ? <Spinner className="size-3.5" /> : leadingIcon}
      {children}
    </button>
  );
}
