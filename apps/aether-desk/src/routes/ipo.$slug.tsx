import { Link, createFileRoute } from "@tanstack/react-router";
import { Bookmark } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Sparkline } from "@/components/sparkline";
import { SubBars } from "@/components/sub-bars";
import { getIpo, expectedList, gmpPct, minInvest } from "@/lib/ipo-data";
import { formatDate, inr } from "@/lib/utils";
import { useWatchlist } from "@/lib/watchlist";

export const Route = createFileRoute("/ipo/$slug")({ component: Detail });

function Detail() {
  const { slug } = Route.useParams();
  const ipo = getIpo(slug);
  const watch = useWatchlist();
  if (!ipo) {
    return (
      <div className="grid gap-4">
        <h1 className="font-display text-3xl">Issue not on the desk</h1>
        <Link to="/" className="text-accent">
          Back to desk
        </Link>
      </div>
    );
  }
  const saved = watch.slugs.includes(ipo.slug);
  const pct = gmpPct(ipo);
  return (
    <div className="grid gap-8 pb-8">
      <div>
        <Link to="/books" className="text-sm text-muted hover:text-fg">
          Desk
        </Link>
        <div className="mt-4 flex flex-wrap items-start justify-between gap-4">
          <div>
            <h1 className="font-display text-4xl tracking-tight">{ipo.name}</h1>
            <p className="mt-2 text-muted">
              {ipo.ticker} · {ipo.board} · {ipo.exchanges} · {ipo.sector}
            </p>
            <div className="mt-3 flex flex-wrap gap-2">
              <Badge tone={ipo.status === "open" ? "open" : ipo.status === "listed" ? "listed" : "neutral"}>
                {ipo.status}
              </Badge>
              {ipo.board === "SME" ? <Badge>SME</Badge> : null}
            </div>
          </div>
          <Button asChild>
            <Link to="/invest" search={{ slug: ipo.slug }}>
              Invest
            </Link>
          </Button>
          <Button variant={saved ? "outline" : "ghost"} onClick={() => watch.toggle(ipo.slug)}>
            <Bookmark className="size-4" fill={saved ? "currentColor" : "none"} />
            {saved ? "Watching" : "Watch"}
          </Button>
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Tile label="Price band" value={`${inr(ipo.priceLow)} – ${inr(ipo.priceHigh)}`} />
        <Tile label="Lot / min" value={`${ipo.lot} sh · ${inr(minInvest(ipo))}`} />
        <Tile
          label="GMP"
          value={`${ipo.gmp >= 0 ? "+" : ""}${inr(ipo.gmp)} (${pct.toFixed(1)}%)`}
          tone={ipo.gmp > 0 ? "gain" : ipo.gmp < 0 ? "loss" : undefined}
        />
        <Tile label="Implied list" value={inr(expectedList(ipo))} />
      </div>

      <div className="grid gap-4 lg:grid-cols-[1.4fr_1fr]">
        <section className="rounded-xl bg-surface p-5">
          <h2 className="font-display text-xl">Subscription</h2>
          <p className="mt-1 text-sm text-muted">Category books, times subscribed.</p>
          <div className="mt-5">
            <SubBars sub={ipo.subscription} />
          </div>
        </section>
        <section className="rounded-xl bg-surface p-5">
          <div className="flex items-center justify-between">
            <h2 className="font-display text-xl">GMP trail</h2>
            <Sparkline values={ipo.gmpHistory} />
          </div>
          <ol className="mt-4 grid gap-2">
            {ipo.gmpHistory.map((g, i) => (
              <li key={i} className="flex justify-between font-mono text-sm tabular-nums text-muted">
                <span>T−{ipo.gmpHistory.length - 1 - i}</span>
                <span className={g > 0 ? "text-gain" : g < 0 ? "text-loss" : ""}>
                  {g >= 0 ? "+" : ""}
                  {inr(g)}
                </span>
              </li>
            ))}
          </ol>
        </section>
      </div>

      <section className="rounded-xl bg-surface p-5">
        <h2 className="font-display text-xl">Calendar</h2>
        <dl className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          <Row k="Open" v={formatDate(ipo.open)} />
          <Row k="Close" v={formatDate(ipo.close)} />
          <Row k="Allotment" v={formatDate(ipo.allotment)} />
          <Row k="Listing" v={formatDate(ipo.listing)} />
        </dl>
      </section>

      <section className="grid gap-4 lg:grid-cols-2">
        <div className="rounded-xl bg-surface p-5">
          <h2 className="font-display text-xl">The company</h2>
          <p className="mt-3 text-muted">{ipo.about}</p>
          <ul className="mt-4 grid gap-2 text-sm text-fg">
            {ipo.objects.map((o) => (
              <li key={o} className="border-l border-accent pl-3">
                {o}
              </li>
            ))}
          </ul>
          <p className="mt-4 text-sm text-muted">
            Issue {inr(ipo.issueCr * 1e7, { compact: true })} · Fresh{" "}
            {inr(ipo.freshCr * 1e7, { compact: true })} · OFS {inr(ipo.ofsCr * 1e7, { compact: true })}
            {ipo.pe ? ` · P/E ${ipo.pe.toFixed(1)}x` : ""}
          </p>
          <a
            href={ipo.rhp}
            target="_blank"
            rel="noreferrer"
            className="mt-4 inline-block text-sm text-accent"
          >
            Registrar {ipo.registrar} · RHP
          </a>
        </div>
        <div className="rounded-xl bg-surface p-5">
          <h2 className="font-display text-xl">Financials</h2>
          <table className="mt-4 w-full text-sm">
            <thead className="text-left text-xs uppercase tracking-wider text-subtle">
              <tr>
                <th className="pb-2 font-medium">Year</th>
                <th className="pb-2 font-medium">Revenue</th>
                <th className="pb-2 font-medium">PAT</th>
              </tr>
            </thead>
            <tbody>
              {ipo.financials.map((f) => (
                <tr key={f.fy} className="border-t border-line">
                  <td className="py-2">{f.fy}</td>
                  <td className="py-2 font-mono tabular-nums">{inr(f.revenueCr * 1e7, { compact: true })}</td>
                  <td className="py-2 font-mono tabular-nums">{inr(f.patCr * 1e7, { compact: true })}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}

function Tile({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "gain" | "loss";
}) {
  return (
    <div className="rounded-lg bg-raised p-4">
      <p className="text-xs text-subtle">{label}</p>
      <p
        className={`mt-2 font-display text-xl tracking-tight ${tone === "gain" ? "text-gain" : tone === "loss" ? "text-loss" : ""}`}
      >
        {value}
      </p>
    </div>
  );
}

function Row({ k, v }: { k: string; v: string }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wider text-subtle">{k}</dt>
      <dd className="mt-1 font-medium">{v}</dd>
    </div>
  );
}
