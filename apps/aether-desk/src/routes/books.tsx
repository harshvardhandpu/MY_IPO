import { useMemo, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { IpoCard } from "@/components/ipo-card";
import { Input } from "@/components/ui/input";
import { PageHeader } from "@/components/page-header";
import { IPOS, type Board, type IpoStatus } from "@/lib/ipo-data";
import { useWatchlist } from "@/lib/watchlist";

export const Route = createFileRoute("/books")({ component: Books });

const filters: { id: "all" | IpoStatus | "watch"; label: string }[] = [
  { id: "all", label: "All" },
  { id: "open", label: "Open" },
  { id: "upcoming", label: "Upcoming" },
  { id: "listed", label: "Listing" },
  { id: "watch", label: "Watchlist" },
];

function Books() {
  const [q, setQ] = useState("");
  const [tab, setTab] = useState<(typeof filters)[number]["id"]>("all");
  const [board, setBoard] = useState<"all" | Board>("all");
  const watch = useWatchlist();

  const list = useMemo(() => {
    return IPOS.filter((i) => {
      if (board !== "all" && i.board !== board) return false;
      if (tab === "watch") return watch.slugs.includes(i.slug);
      if (tab !== "all" && i.status !== tab) return false;
      if (q.trim()) {
        const s = q.toLowerCase();
        return (
          i.name.toLowerCase().includes(s) ||
          i.ticker.toLowerCase().includes(s) ||
          i.sector.toLowerCase().includes(s)
        );
      }
      return true;
    });
  }, [q, tab, board, watch.slugs]);

  return (
    <div className="grid gap-6 pb-8">
      <PageHeader
        kicker="Catalogue"
        title="Open books"
        body="Public issue list with GMP, lot, and registrar. Watchlist is local to this desk."
      />
      <Input
        value={q}
        onChange={(e) => setQ(e.target.value)}
        placeholder="Search name, ticker, sector"
        aria-label="Search IPOs"
      />
      <div className="flex flex-wrap gap-2">
        {(["all", "Mainboard", "SME"] as const).map((b) => (
          <button
            key={b}
            type="button"
            onClick={() => setBoard(b)}
            className={`h-11 rounded-full px-4 text-sm ${board === b ? "bg-raised text-fg" : "text-muted"}`}
          >
            {b === "all" ? "Both" : b}
          </button>
        ))}
      </div>
      <div className="flex flex-wrap gap-1">
        {filters.map((f) => (
          <button
            key={f.id}
            type="button"
            onClick={() => setTab(f.id)}
            className={`h-11 rounded-full px-4 text-sm ${tab === f.id ? "bg-fg text-bg" : "text-muted hover:text-fg"}`}
          >
            {f.label}
          </button>
        ))}
      </div>
      {list.length === 0 ? (
        <p className="rounded-xl bg-surface px-4 py-10 text-center text-muted">
          Nothing in this slice.
        </p>
      ) : (
        <div className="grid gap-3">
          {list.map((ipo) => (
            <IpoCard key={ipo.slug} ipo={ipo} />
          ))}
        </div>
      )}
    </div>
  );
}
