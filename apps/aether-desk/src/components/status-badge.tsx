import {
  Ban,
  Check,
  CircleHelp,
  Clock,
  Minus,
  PlugZap,
  SearchX,
  ShieldQuestion,
  Timer,
} from "lucide-react";
import { cn } from "@/lib/utils";

const copy: Record<string, { label: string; tone: string; Icon: typeof Check }> = {
  PENDING: { label: "Pending", tone: "text-muted bg-raised", Icon: Clock },
  CHECKING: { label: "Checking", tone: "text-warn bg-warn/15", Icon: Timer },
  READY: { label: "Ready to check", tone: "text-muted bg-raised", Icon: Clock },
  PARTIAL: { label: "Partially complete", tone: "text-warn bg-warn/15", Icon: Timer },
  ALLOTTED: { label: "Allotted", tone: "text-gain bg-gain/15", Icon: Check },
  NOT_ALLOTTED: { label: "Not allotted", tone: "text-fg bg-raised", Icon: Minus },
  NOT_FOUND: { label: "Not found", tone: "text-warn bg-warn/15", Icon: SearchX },
  UNKNOWN: { label: "Unknown", tone: "text-fund bg-fund/15", Icon: CircleHelp },
  NEEDS_HUMAN: { label: "Verification required", tone: "text-warn bg-warn/15", Icon: ShieldQuestion },
  RATE_LIMITED: { label: "Rate limited", tone: "text-warn bg-warn/15", Icon: Timer },
  PROVIDER_UNAVAILABLE: { label: "Provider unavailable", tone: "text-loss bg-loss/15", Icon: PlugZap },
  MANUAL: { label: "Manual result", tone: "text-gain bg-gain/15", Icon: Check },
  CANCELLED: { label: "Voided", tone: "text-subtle bg-raised", Icon: Ban },
};

export function StatusBadge({ status }: { status: string }) {
  const item = copy[status] ?? {
    label: status.replaceAll("_", " "),
    tone: "text-muted bg-raised",
    Icon: CircleHelp,
  };
  const Icon = item.Icon;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-sm px-2 py-0.5 text-xs font-medium",
        item.tone,
      )}
    >
      <Icon className="size-3" aria-hidden />
      {item.label}
    </span>
  );
}
