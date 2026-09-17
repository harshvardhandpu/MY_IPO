import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function Badge({
  className,
  tone = "neutral",
  children,
}: {
  className?: string;
  tone?: "neutral" | "open" | "up" | "down" | "listed" | "sme";
  children: ReactNode;
}) {
  const tones = {
    neutral: "bg-raised text-muted",
    open: "bg-accent/15 text-accent",
    up: "bg-gain/15 text-gain",
    down: "bg-loss/15 text-loss",
    listed: "bg-warn/15 text-warn",
    sme: "bg-raised text-fg",
  };
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-sm px-2 py-0.5 text-xs font-medium tracking-wide",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}
