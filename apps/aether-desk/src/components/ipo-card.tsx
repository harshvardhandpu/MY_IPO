import { Link } from "@tanstack/react-router";
import { Bookmark } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Sparkline } from "@/components/sparkline";
import type { Ipo } from "@/lib/ipo-data";
import { expectedList, gmpPct, minInvest } from "@/lib/ipo-data";
import { formatShort, inr } from "@/lib/utils";
import { useWatchlist } from "@/lib/watchlist";
import { cn } from "@/lib/utils";

const statusTone = {
  open: "open" as const,
  upcoming: "neutral" as const,
  closed: "neutral" as const,
  listed: "listed" as const,
};

export function IpoCard({ ipo }: { ipo: Ipo }) {
  const watch = useWatchlist();
  const saved = watch.slugs.includes(ipo.slug);
  const pct = gmpPct(ipo);
  return (
    <article className="rounded-xl bg-surface p-4">
      <div className="flex items-start justify-between gap-3">
        <Link to="/ipo/$slug" params={{ slug: ipo.slug }} className="min-w-0">
          <p className="text-lg font-medium leading-tight tracking-tight">{ipo.name}</p>
          <p className="mt-1 text-xs text-muted">
            {ipo.board} · {ipo.exchanges} · {ipo.sector}
          </p>
        </Link>
        <button
          type="button"
          aria-label={saved ? "Remove from watchlist" : "Watch"}
          onClick={() => watch.toggle(ipo.slug)}
          className={cn(
            "flex size-11 items-center justify-center rounded-full",
            saved ? "text-accent" : "text-subtle hover:text-fg",
          )}
        >
          <Bookmark className="size-4" fill={saved ? "currentColor" : "none"} />
        </button>
      </div>
      <div className="mt-4 flex flex-wrap items-center gap-2">
        <Badge tone={statusTone[ipo.status]}>{ipo.status}</Badge>
        {ipo.board === "SME" ? <Badge tone="sme">SME</Badge> : null}
      </div>
      <div className="mt-5 grid grid-cols-2 gap-4 sm:grid-cols-4">
        <Stat label="Band" value={`${inr(ipo.priceLow)}–${inr(ipo.priceHigh)}`} />
        <Stat label="Lot" value={`${ipo.lot} · ${inr(minInvest(ipo), { compact: true })}`} />
        <Stat
          label="GMP"
          value={`${ipo.gmp >= 0 ? "+" : ""}${inr(ipo.gmp)}`}
          tone={ipo.gmp > 0 ? "up" : ipo.gmp < 0 ? "down" : undefined}
        />
        <Stat
          label="Implied"
          value={`${inr(expectedList(ipo))} (${pct >= 0 ? "+" : ""}${pct.toFixed(1)}%)`}
          tone={pct > 0 ? "up" : pct < 0 ? "down" : undefined}
        />
      </div>
      <div className="mt-4 flex items-center justify-between text-xs text-muted">
        <span>
          Closes {formatShort(ipo.close)} · lists {formatShort(ipo.listing)}
        </span>
        <Sparkline values={ipo.gmpHistory} />
      </div>
    </article>
  );
}

function Stat({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "up" | "down";
}) {
  return (
    <div>
      <p className="text-xs text-subtle">{label}</p>
      <p
        className={cn(
          "mt-1 text-sm tabular-nums",
          tone === "up" && "text-gain",
          tone === "down" && "text-loss",
        )}
      >
        {value}
      </p>
    </div>
  );
}
