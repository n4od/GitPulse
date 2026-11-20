import type { ComponentPropsWithRef } from "react";
import { cn } from "@/lib/cn";

export function Card({ className, children, ...rest }: ComponentPropsWithRef<"div">) {
  return (
    <div className={cn("rounded-card border border-line bg-raised", className)} {...rest}>
      {children}
    </div>
  );
}

export function CardHeader({
  className,
  children,
  ...rest
}: ComponentPropsWithRef<"div">) {
  return (
    <div
      className={cn("flex items-center gap-2 border-b border-line px-4 py-3", className)}
      {...rest}
    >
      {children}
    </div>
  );
}

export function CardBody({ className, children, ...rest }: ComponentPropsWithRef<"div">) {
  return (
    <div className={cn("px-4 py-3", className)} {...rest}>
      {children}
    </div>
  );
}
