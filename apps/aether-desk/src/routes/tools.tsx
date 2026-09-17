import { useMemo, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Input } from "@/components/ui/input";
import { IPOS, expectedList, minInvest } from "@/lib/ipo-data";
import { inr } from "@/lib/utils";

export const Route = createFileRoute("/tools")({ component: Tools });

function Tools() {
  const [slug, setSlug] = useState(IPOS[0].slug);
  const [lots, setLots] = useState(1);
  const ipo = useMemo(() => IPOS.find((i) => i.slug === slug) ?? IPOS[0], [slug]);
  const applyAmt = minInvest(ipo) * lots;
  const listAmt = expectedList(ipo) * ipo.lot * lots;
  const pnl = listAmt - applyAmt;
  const retailOdds = Math.min(99, Math.max(4, Math.round(100 / Math.max(1, ipo.subscription.retail || 1))));

  return (
    <div className="grid max-w-2xl gap-8 pb-8">
      <header>
        <p className="text-xs text-muted">Sizing</p>
        <h1 className="mt-2 font-display text-3xl tracking-tight">Lot calculator</h1>
        <p className="mt-3 text-muted">
          Apply amount, implied listing, and a rough retail allotment sketch from the live book. Not advice.
        </p>
      </header>
      <div className="grid gap-4 rounded-xl bg-surface p-5">
        <label className="grid gap-2 text-sm">
          Issue
          <select
            value={slug}
            onChange={(e) => setSlug(e.target.value)}
            className="h-11 rounded-md border border-line bg-surface px-3 text-sm"
          >
            {IPOS.map((i) => (
              <option key={i.slug} value={i.slug}>
                {i.name}
              </option>
            ))}
          </select>
        </label>
        <label className="grid gap-2 text-sm">
          Lots
          <Input
            type="number"
            min={1}
            max={99}
            value={lots}
            onChange={(e) => setLots(Math.max(1, Number(e.target.value) || 1))}
          />
        </label>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <Stat label="Apply amount" value={inr(applyAmt)} />
        <Stat label="Implied listing" value={inr(listAmt)} />
        <Stat
          label="Paper P/L"
          value={`${pnl >= 0 ? "+" : ""}${inr(pnl)}`}
          good={pnl >= 0}
        />
        <Stat label="Retail sketch" value={`${retailOdds}% one lot`} />
      </div>
      <p className="text-xs text-subtle">
        GMP is an unofficial grey-market quote. Listing can print far from it. STT, brokerage, and tax are not included.
      </p>
    </div>
  );
}

function Stat({
  label,
  value,
  good,
}: {
  label: string;
  value: string;
  good?: boolean;
}) {
  return (
    <div className="rounded-lg border border-line bg-surface p-4">
      <p className="text-xs uppercase tracking-wider text-subtle">{label}</p>
      <p className={`mt-2 font-display text-2xl ${good === false ? "text-loss" : good ? "text-gain" : ""}`}>
        {value}
      </p>
    </div>
  );
}
