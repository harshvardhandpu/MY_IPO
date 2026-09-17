import { useMemo, useState } from "react";
import { Link, createFileRoute } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { PageHeader, Panel, SecurityStrip } from "@/components/page-header";
import { StatusBadge } from "@/components/status-badge";
import { getIpo } from "@/lib/ipo-data";
import {
  estimatedProfitPaise,
  jobState,
  maskLast4,
  officialUrl,
  plannedPaise,
  providerForRegistrar,
  useLedger,
} from "@/lib/ledger";
import { inr } from "@/lib/utils";

export const Route = createFileRoute("/allotment")({ component: Allotment });

function Allotment() {
  const applications = useLedger((s) => s.applications);
  const accounts = useLedger((s) => s.accounts);
  const runAllotment = useLedger((s) => s.runAllotment);
  const setManualResult = useLedger((s) => s.setManualResult);
  const live = useMemo(() => applications.filter((a) => !a.voided), [applications]);
  const [selected, setSelected] = useState(live[0]?.id ?? "");
  const [manualLots, setManualLots] = useState(1);
  const app = live.find((a) => a.id === selected) ?? live[0];
  const ipo = app ? getIpo(app.ipoSlug) : undefined;
  const provider = app ? providerForRegistrar(app.registrar) : null;
  const official = app ? officialUrl(app.registrar) : "";

  if (!live.length) {
    return (
      <div className="grid max-w-xl gap-4 pb-8">
        <PageHeader
          kicker="Registrar workflow"
          title="Check allotment"
          body="No submitted IPO applications yet."
        />
        <Button asChild>
          <Link to="/invest">Invest first</Link>
        </Button>
      </div>
    );
  }

  const current = app!;
  const human = current.allocations.some((a) => a.status === "NEEDS_HUMAN");
  const profit = estimatedProfitPaise(current);
  const state = jobState(current);

  return (
    <div className="grid gap-6 pb-8">
      <PageHeader
        kicker="Registrar workflow"
        title="Check allotment"
        body="Simulated, fail-closed lookup. PAN is never typed here. 404 stays unresolved — never not allotted. Bigshare always pauses for human verification."
      />
      <SecurityStrip>Real PAN lookup is not authorized on this desk.</SecurityStrip>

      <div className="grid gap-6 lg:grid-cols-[0.9fr_1.1fr]">
        <div className="grid gap-2">
          {live.map((row) => {
            const name = getIpo(row.ipoSlug)?.name ?? row.ipoSlug;
            const prov = providerForRegistrar(row.registrar);
            return (
              <label
                key={row.id}
                className="flex min-h-11 items-center gap-3 rounded-xl bg-surface px-4 py-3"
              >
                <input
                  type="radio"
                  name="app"
                  checked={current.id === row.id}
                  onChange={() => setSelected(row.id)}
                />
                <span className="min-w-0 flex-1">
                  <span className="block font-medium">{name}</span>
                  <span className="text-xs text-muted">
                    {prov.label} · {row.allocations.length} accounts · {inr(plannedPaise(row) / 100)}
                  </span>
                </span>
                <StatusBadge status={jobState(row)} />
              </label>
            );
          })}
        </div>

        <Panel>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p className="text-xs text-muted">{current.registrar}</p>
              <h2 className="mt-1 font-medium">{ipo?.name ?? current.ipoSlug}</h2>
              <p className="mt-1 text-xs text-subtle">
                Provider {provider?.label}
                {provider?.auto ? " · automatic transport" : " · human verification"} · {state}
              </p>
            </div>
            <StatusBadge status={state} />
          </div>
          <p className="mt-4 text-sm text-muted">
            Estimated profit{" "}
            {profit == null ? "not available yet" : inr(profit / 100)}
            {profit != null ? " · GMP × allotted shares, friend share only on positive profit" : ""}
          </p>
          <div className="mt-4 flex flex-wrap gap-2">
            <Button onClick={() => runAllotment(current.id)}>Check all accounts</Button>
            <Button variant="outline" asChild>
              <a href={official} target="_blank" rel="noreferrer">
                Open official page
              </a>
            </Button>
            <Button variant="ghost" asChild>
              <Link to="/investments">Ledger</Link>
            </Button>
          </div>
        </Panel>
      </div>

      {human ? (
        <section className="rounded-xl bg-warn/10 p-5">
          <h2 className="font-medium">Verification required</h2>
          <p className="mt-2 text-sm text-muted">
            The official registrar requires a human step. This desk will not bypass it or mark the
            account as not allotted.
          </p>
        </section>
      ) : null}

      <section className="overflow-x-auto rounded-xl bg-surface">
        <table className="w-full min-w-[40rem] text-left text-sm">
          <thead className="text-xs uppercase tracking-wider text-subtle">
            <tr>
              <th className="px-4 py-3 font-medium">Account</th>
              <th className="px-4 py-3 font-medium">Amount</th>
              <th className="px-4 py-3 font-medium">Status</th>
              <th className="px-4 py-3 font-medium">Lots</th>
              <th className="px-4 py-3 font-medium">Provenance</th>
            </tr>
          </thead>
          <tbody>
            {current.allocations.map((row) => {
              const acct = accounts.find((a) => a.id === row.accountId);
              const canManual =
                row.status === "NEEDS_HUMAN" ||
                row.status === "UNKNOWN" ||
                row.status === "PENDING" ||
                row.status === "NOT_FOUND" ||
                row.status === "RATE_LIMITED";
              return (
                <tr key={row.id} className="border-t border-line">
                  <td className="px-4 py-3">
                    <p className="font-medium">{acct?.label ?? "Account"}</p>
                    <p className="text-xs text-muted">
                      {acct?.kind} · {maskLast4(acct?.last4 ?? "")}
                    </p>
                  </td>
                  <td className="px-4 py-3 tabular-nums">{inr(row.amountPaise / 100)}</td>
                  <td className="px-4 py-3">
                    <StatusBadge status={row.status} />
                  </td>
                  <td className="px-4 py-3 tabular-nums">
                    {row.allottedShares > 0
                      ? `${row.allottedLots} lot · ${row.allottedShares} sh`
                      : "—"}
                  </td>
                  <td className="px-4 py-3">
                    <p className="text-xs">{row.provenance.replaceAll("_", " ")}</p>
                    <p className="text-xs text-subtle">{row.note}</p>
                    {canManual ? (
                      <div className="mt-2 flex flex-wrap gap-3">
                        <button
                          type="button"
                          className="text-xs text-gain"
                          onClick={() => setManualResult(current.id, row.id, "ALLOTTED", manualLots)}
                        >
                          Enter allotted
                        </button>
                        <button
                          type="button"
                          className="text-xs text-muted"
                          onClick={() =>
                            setManualResult(current.id, row.id, "NOT_ALLOTTED", 0)
                          }
                        >
                          Manual not allotted
                        </button>
                      </div>
                    ) : null}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </section>

      <label className="flex items-center gap-2 text-sm text-muted">
        Manual lots
        <input
          type="number"
          min={1}
          className="h-11 w-20 rounded-md border border-line bg-raised px-2"
          value={manualLots}
          onChange={(e) => setManualLots(Math.max(1, Number(e.target.value)))}
        />
        {ipo ? <span>· {ipo.lot} shares / lot</span> : null}
      </label>
    </div>
  );
}
