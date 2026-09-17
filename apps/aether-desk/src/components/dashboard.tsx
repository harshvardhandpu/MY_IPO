import { useMemo, useState } from "react";
import { Link } from "@tanstack/react-router";
import { Area, AreaChart, ResponsiveContainer, Tooltip } from "recharts";
import { ChevronRight } from "lucide-react";
import { StatusBadge } from "@/components/status-badge";
import { IPOS, gmpPct, getIpo, minInvest, type Ipo } from "@/lib/ipo-data";
import {
  estimatedProfitPaise,
  jobState,
  plannedPaise,
  summarizeDesk,
  useLedger,
} from "@/lib/ledger";
import { useWatchlist } from "@/lib/watchlist";
import { useWidgets, type WidgetId } from "@/lib/widgets";
import { cn, hashSeed, inr } from "@/lib/utils";

const RANGES = ["1D", "7D", "1M", "1Y", "All"] as const;
const HEAT = ["heat-0", "heat-1", "heat-2", "heat-3", "heat-4"] as const;

export function Dashboard() {
  const widgets = useWidgets();
  const show = (id: WidgetId) => widgets.visible(id);
  return (
    <div className="desk">
      {show("sentiment") ? <SentimentCard /> : null}
      {show("performance") ? <PerformanceCard /> : null}
      {show("portfolio") ? <PortfolioCard /> : null}
      {show("protect") ? <ProtectCard /> : null}
      {show("heatmap") ? <HeatmapCard /> : null}
      {show("quick") ? <QuickCard /> : null}
      {show("picks") ? <PicksCard /> : null}
      {show("activity") ? <ActivityCard /> : null}
    </div>
  );
}

function Panel({
  area,
  className,
  children,
}: {
  area: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <section className={cn("rounded-xl bg-surface p-5", area, className)}>{children}</section>
  );
}

function SentimentCard() {
  const score = useMemo(() => {
    const open = IPOS.filter((i) => i.status === "open");
    if (!open.length) return 50;
    const raw = open.reduce((s, i) => {
      const g = Math.max(-20, Math.min(40, gmpPct(i)));
      const book = Math.min(30, i.subscription.total * 6);
      return s + 45 + g + book;
    }, 0);
    return Math.max(8, Math.min(96, Math.round(raw / open.length)));
  }, []);
  return (
    <Panel area="area-sent">
      <div className="flex items-start justify-between gap-3">
        <h2 className="text-sm text-muted">Market sentiment</h2>
        <p className="font-display text-3xl font-medium tabular-nums tracking-tight">{score}%</p>
      </div>
      <svg viewBox="0 0 240 150" className="mx-auto mt-2 h-36 w-full" aria-hidden>
        <path
          d="M22 132 A98 98 0 0 1 218 132"
          fill="none"
          stroke="var(--color-raised)"
          strokeWidth="22"
          strokeLinecap="round"
        />
        <path
          d="M22 132 A98 98 0 0 1 218 132"
          fill="none"
          stroke="var(--color-gain)"
          strokeWidth="22"
          strokeLinecap="round"
          pathLength="100"
          strokeDasharray={`${score} ${100 - score}`}
        />
      </svg>
      <p className="text-center text-sm text-muted">
        Open books lean {score >= 55 ? "towards subscription" : "cautious"} on GMP and demand.
      </p>
    </Panel>
  );
}

function PerformanceCard() {
  const [range, setRange] = useState<(typeof RANGES)[number]>("1M");
  const applications = useLedger((s) => s.applications);
  const data = useMemo(() => buildSeries(range, applications), [range, applications]);
  const last = data[data.length - 1]?.v ?? 0;

  return (
    <Panel area="area-perf">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-sm text-muted">Book performance</h2>
        <div className="flex rounded-full bg-raised p-1">
          {RANGES.map((r) => (
            <button
              key={r}
              type="button"
              onClick={() => setRange(r)}
              className={cn(
                "h-8 rounded-full px-3 text-xs",
                range === r ? "bg-window text-fg" : "text-muted",
              )}
            >
              {r}
            </button>
          ))}
        </div>
      </div>
      <div className="relative mt-4 h-40">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={data}>
            <Tooltip
              cursor={{ stroke: "var(--color-line)" }}
              contentStyle={{
                background: "var(--color-window)",
                border: "1px solid var(--color-line)",
                borderRadius: 14,
                color: "var(--color-fg)",
              }}
              formatter={(value: number) => [inr(value), ""]}
            />
            <Area
              type="monotone"
              dataKey="v"
              stroke="var(--color-fg)"
              strokeWidth={2}
              fill="none"
              dot={false}
              activeDot={{ r: 5, fill: "var(--color-window)", stroke: "var(--color-fg)" }}
            />
          </AreaChart>
        </ResponsiveContainer>
        <p className="pointer-events-none absolute left-1/2 top-8 -translate-x-1/2 rounded-full bg-window px-3 py-1 text-xs tabular-nums">
          {inr(last)}
        </p>
      </div>
    </Panel>
  );
}

function PortfolioCard() {
  const applications = useLedger((s) => s.applications);
  const accounts = useLedger((s) => s.accounts);
  const watch = useWatchlist();
  const summary = useMemo(
    () => summarizeDesk(applications, accounts),
    [applications, accounts],
  );
  const held = useMemo(() => {
    const fromLedger = summary.live
      .map((a) => getIpo(a.ipoSlug))
      .filter((i): i is Ipo => Boolean(i));
    if (fromLedger.length) return uniqueIpos(fromLedger);
    const list = IPOS.filter((i) => watch.slugs.includes(i.slug));
    return list.length ? list : IPOS.slice(0, 4);
  }, [summary.live, watch.slugs]);
  const cost = summary.sessions
    ? summary.plannedPaise / 100
    : held.reduce((s, i) => s + minInvest(i), 0);
  const profit = summary.profitPaise != null ? summary.profitPaise / 100 : held.reduce((s, i) => s + i.gmp * i.lot, 0);
  const pct = cost ? (profit / cost) * 100 : 0;
  const sectors = groupSectors(held, summary.live);
  const spark = held[0]?.gmpHistory.map((g, i) => ({ i, v: 80 + g })) ?? [];

  return (
    <Panel area="area-port">
      <div className="flex items-center justify-between">
        <h2 className="text-sm text-muted">My book</h2>
        <Link to="/invest" className="rounded-full bg-raised px-3 py-1 text-xs">
          Invest
        </Link>
      </div>
      <p className="mt-4 font-display text-3xl font-medium tabular-nums tracking-tight">
        {inr(cost)}
      </p>
      <p className="mt-2 text-sm">
        <span className={pct >= 0 ? "text-gain" : "text-loss"}>
          {pct >= 0 ? "+" : ""}
          {pct.toFixed(1)}%
        </span>
        <span className="ml-2 text-muted">
          {summary.profitPaise != null ? "estimated on allotted GMP" : "versus issue price"}
        </span>
      </p>
      <div className="mt-3 h-20">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={spark}>
            <Area type="monotone" dataKey="v" stroke="var(--color-fg)" strokeWidth={2} fill="none" />
          </AreaChart>
        </ResponsiveContainer>
      </div>
      <div className="mt-2 flex h-2 overflow-hidden rounded-full">
        {sectors.map((s) => (
          <span key={s.name} className={cn("h-full", s.bar)} style={{ width: `${s.pct}%` }} />
        ))}
      </div>
      <ul className="mt-4 grid gap-2">
        {sectors.map((s) => (
          <li key={s.name} className="flex items-center justify-between text-sm">
            <span className="flex items-center gap-2 text-muted">
              <span className={cn("size-2 rounded-full", s.bar)} />
              {s.name}
              <span className="text-subtle">{s.share}%</span>
            </span>
            <span className="tabular-nums">{inr(s.value)}</span>
          </li>
        ))}
      </ul>
      <Link to="/investments" className="mt-5 block text-center text-sm text-muted hover:text-fg">
        View all {summary.sessions || held.length}
      </Link>
    </Panel>
  );
}

function ProtectCard() {
  const applications = useLedger((s) => s.applications);
  const pending = applications.filter(
    (a) => !a.voided && a.allocations.some((r) => r.status === "PENDING" || r.status === "NEEDS_HUMAN"),
  ).length;
  return (
    <Link
      to="/allotment"
      className="area-prot relative overflow-hidden rounded-xl bg-accent p-5 text-accent-fg"
    >
      <p className="inline-flex items-center gap-2 rounded-full bg-accent-fg/10 px-2 py-1 text-xs">
        {pending ? `${pending} ready to check` : "Protection level: Basic"}
      </p>
      <h2 className="mt-5 max-w-[14rem] font-display text-3xl font-medium leading-tight tracking-tight">
        Check allotment
      </h2>
      <p className="mt-3 max-w-xs text-sm text-accent-fg/80">
        Fail-closed registrar lookup. 404 stays unresolved. Bigshare always pauses for a human.
      </p>
      <span className="mt-8 inline-flex h-11 items-center rounded-full bg-accent-fg px-4 text-sm font-medium text-accent">
        Open queue
        <ChevronRight className="ml-1 size-4" />
      </span>
      <svg
        className="pointer-events-none absolute -bottom-4 right-2 h-36 w-36"
        viewBox="0 0 120 120"
        aria-hidden
      >
        <rect x="30" y="54" width="60" height="46" rx="14" fill="var(--color-window)" />
        <path
          d="M46 54v-10a14 14 0 0 1 28 0v10"
          fill="none"
          stroke="var(--color-window)"
          strokeWidth="10"
          strokeLinecap="round"
        />
        <circle cx="48" cy="78" r="4.5" fill="var(--color-muted)" />
        <circle cx="60" cy="78" r="4.5" fill="var(--color-muted)" />
        <circle cx="72" cy="78" r="4.5" fill="var(--color-muted)" />
      </svg>
    </Link>
  );
}

function HeatmapCard() {
  const cells = useMemo(() => {
    const grid = Array.from({ length: 7 * 24 }, () => 0);
    for (const ipo of IPOS) {
      for (const key of ["open", "close", "allotment", "listing"] as const) {
        const d = new Date(`${ipo[key]}T00:00:00`);
        const col = Math.min(23, Math.max(0, d.getDate() - 1));
        const row = d.getDay();
        const i = row * 24 + col;
        grid[i] = Math.min(4, grid[i] + 1);
      }
    }
    return grid;
  }, []);
  return (
    <Panel area="area-heat">
      <h2 className="text-sm text-muted">Issue calendar</h2>
      <div className="mt-5 overflow-x-auto">
        <div className="grid w-max grid-cols-[repeat(24,minmax(0,1fr))] grid-rows-7 gap-1">
          {cells.map((v, i) => (
            <span key={i} className={cn("size-4 rounded-xs", HEAT[v])} />
          ))}
        </div>
        <div className="mt-3 flex justify-between text-xs text-subtle">
          {["Open", "Close", "Allot", "List", "Sep"].map((m) => (
            <span key={m}>{m}</span>
          ))}
        </div>
      </div>
    </Panel>
  );
}

function QuickCard() {
  const items = [
    { to: "/invest", label: "Invest" },
    { to: "/allotment", label: "Check allotment" },
    { to: "/members", label: "Members" },
    { to: "/investments", label: "Ledger" },
  ] as const;
  return (
    <Panel area="area-quic">
      <h2 className="text-sm text-muted">Quick actions</h2>
      <div className="mt-4 flex flex-wrap gap-2">
        {items.map((item) => (
          <Link
            key={item.to}
            to={item.to}
            className="flex h-11 items-center rounded-full bg-raised px-4 text-sm"
          >
            {item.label}
          </Link>
        ))}
      </div>
    </Panel>
  );
}

function PicksCard() {
  const picks = [...IPOS]
    .filter((i) => i.status === "open" || i.status === "upcoming")
    .sort((a, b) => gmpPct(b) - gmpPct(a))
    .slice(0, 6);
  return (
    <Panel area="area-pick">
      <div className="flex items-center justify-between">
        <h2 className="text-sm text-muted">Top picks</h2>
        <Link to="/books" className="text-xs text-muted hover:text-fg">
          Catalogue
        </Link>
      </div>
      <div className="mt-5 flex gap-4 overflow-x-auto pb-1">
        {picks.map((ipo) => (
          <Link
            key={ipo.slug}
            to="/ipo/$slug"
            params={{ slug: ipo.slug }}
            className="flex w-16 shrink-0 flex-col items-center gap-2"
          >
            <span
              className={cn(
                "flex size-14 items-center justify-center rounded-full text-xs font-medium text-bg",
                avatarTone(ipo.slug),
              )}
            >
              {ipo.ticker.slice(0, 2)}
            </span>
            <span className="w-full truncate text-center text-xs text-muted">
              {ipo.name.split(" ")[0]}
            </span>
          </Link>
        ))}
      </div>
    </Panel>
  );
}

function ActivityCard() {
  const applications = useLedger((s) => s.applications);
  const rows = useMemo(() => {
    const live = applications.filter((a) => !a.voided).slice(0, 4);
    if (live.length) return live;
    return IPOS.filter((i) => i.status === "listed")
      .slice(0, 4)
      .map((ipo) => ({
        id: ipo.slug,
        ipoSlug: ipo.slug,
        source: "CATALOGUE" as const,
        amount: minInvest(ipo),
        status: "LISTED",
      }));
  }, [applications]);

  return (
    <Panel area="area-acti">
      <div className="flex items-center justify-between">
        <h2 className="text-sm text-muted">Last activity</h2>
        <Link to="/investments" className="text-xs text-muted hover:text-fg">
          View all
        </Link>
      </div>
      <ul className="mt-4 grid gap-3">
        {rows.map((row) => {
          const ipo = getIpo(row.ipoSlug);
          if (!ipo) return null;
          const amount =
            "allocations" in row ? plannedPaise(row) / 100 : "amount" in row ? row.amount : 0;
          const status = "allocations" in row ? jobState(row) : "LISTED";
          return (
            <li key={row.id}>
              <Link to="/ipo/$slug" params={{ slug: ipo.slug }} className="flex items-center gap-3">
                <span
                  className={cn(
                    "flex size-10 items-center justify-center rounded-full text-xs font-medium text-bg",
                    avatarTone(ipo.slug),
                  )}
                >
                  {ipo.ticker.slice(0, 2)}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm">{ipo.name}</span>
                  <span className="mt-1 inline-flex">
                    {"allocations" in row ? (
                      <StatusBadge status={status} />
                    ) : (
                      <span className="text-xs text-gain">Listed</span>
                    )}
                  </span>
                </span>
                <span className="text-right text-sm tabular-nums">{inr(-amount)}</span>
              </Link>
            </li>
          );
        })}
      </ul>
    </Panel>
  );
}

function buildSeries(
  range: (typeof RANGES)[number],
  applications: ReturnType<typeof useLedger.getState>["applications"],
) {
  const n = range === "1D" ? 12 : range === "7D" ? 18 : range === "1M" ? 28 : range === "1Y" ? 36 : 42;
  const live = applications.filter((a) => !a.voided);
  const base =
    live.reduce((s, a) => s + plannedPaise(a), 0) / 100 ||
    IPOS.slice(0, 4).reduce((s, i) => s + minInvest(i), 0);
  const est =
    live.reduce((s, a) => s + (estimatedProfitPaise(a) ?? 0), 0) / 100;
  let v = Math.max(1200, base - est);
  return Array.from({ length: n }, (_, i) => {
    v += (hashSeed(`${range}-${i}-${live.length}`) % 720) - 280;
    if (i === n - 1) v = base + est;
    return { i, v: Math.max(0, Math.round(v)) };
  });
}

function uniqueIpos(list: Ipo[]) {
  return [...new Map(list.map((i) => [i.slug, i])).values()];
}

function groupSectors(list: Ipo[], live: ReturnType<typeof summarizeDesk>["live"]) {
  const bars = ["bg-chip", "bg-heat", "bg-fund", "bg-gain", "bg-muted"] as const;
  const map = new Map<string, number>();
  if (live.length) {
    for (const app of live) {
      const ipo = getIpo(app.ipoSlug);
      if (!ipo) continue;
      map.set(ipo.sector, (map.get(ipo.sector) ?? 0) + plannedPaise(app) / 100);
    }
  } else {
    for (const ipo of list) map.set(ipo.sector, (map.get(ipo.sector) ?? 0) + minInvest(ipo));
  }
  const total = [...map.values()].reduce((a, b) => a + b, 0) || 1;
  return [...map.entries()].map(([name, value], i) => ({
    name,
    value,
    pct: (value / total) * 100,
    share: Math.round((value / total) * 100),
    bar: bars[i % bars.length],
  }));
}

function avatarTone(slug: string) {
  const tones = ["bg-chip", "bg-heat", "bg-gain", "bg-fund", "bg-accent"] as const;
  return tones[hashSeed(slug) % tones.length];
}
