import { useMemo, useState } from "react";
import { Link, createFileRoute } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PageHeader, Panel, SecurityStrip } from "@/components/page-header";
import { StatusBadge } from "@/components/status-badge";
import { IPOS, getIpo, minInvest } from "@/lib/ipo-data";
import {
  estimatedProfitPaise,
  jobState,
  plannedPaise,
  useLedger,
} from "@/lib/ledger";
import { formatDate, inr } from "@/lib/utils";

export const Route = createFileRoute("/investments")({ component: Investments });

function Investments() {
  const applications = useLedger((s) => s.applications);
  const accounts = useLedger((s) => s.accounts);
  const submitInvestment = useLedger((s) => s.submitInvestment);
  const voidApplication = useLedger((s) => s.voidApplication);
  const [histSlug, setHistSlug] = useState(IPOS[0].slug);
  const [histLots, setHistLots] = useState(1);
  const [histDate, setHistDate] = useState("2026-08-01");
  const [affirmed, setAffirmed] = useState(false);
  const [msg, setMsg] = useState("");
  const [showHist, setShowHist] = useState(false);
  const owner = useMemo(() => accounts.find((a) => a.kind === "PRIMARY"), [accounts]);
  const live = applications;

  function addHistorical(e: React.FormEvent) {
    e.preventDefault();
    if (!affirmed || !owner) {
      setMsg("Affirm that this is a historical owner-entered fact.");
      return;
    }
    submitInvestment({
      ipoSlug: histSlug,
      accountIds: [owner.id],
      lots: histLots,
      source: "HISTORICAL",
      applicationDate: histDate,
    });
    setAffirmed(false);
    setMsg("Historical application recorded. No registrar lookup was made.");
  }

  return (
    <div className="grid gap-6 pb-8">
      <PageHeader
        kicker="Ledger"
        title="Investments"
        body="Allocation-level truth. Estimated profit uses GMP × allotted shares. Friend share only on positive profit."
        actions={
          <>
            <Button variant="outline" onClick={() => setShowHist((v) => !v)}>
              Historical
            </Button>
            <Button asChild>
              <Link to="/invest">Invest</Link>
            </Button>
          </>
        }
      />
      <SecurityStrip>
        Voiding a session is an owner correction. It does not talk to a registrar.
      </SecurityStrip>

      {live.length === 0 ? (
        <p className="rounded-xl bg-surface px-4 py-10 text-center text-muted">
          No submitted sessions. Preview an investment or add a historical record.
        </p>
      ) : (
        <div className="overflow-x-auto rounded-xl bg-surface">
          <table className="w-full min-w-[44rem] text-left text-sm">
            <thead className="text-xs uppercase tracking-wider text-subtle">
              <tr>
                <th className="px-4 py-3 font-medium">IPO</th>
                <th className="px-4 py-3 font-medium">Source</th>
                <th className="px-4 py-3 font-medium">Status</th>
                <th className="px-4 py-3 font-medium">Amount</th>
                <th className="px-4 py-3 font-medium">Est. profit</th>
                <th className="px-4 py-3 font-medium" />
              </tr>
            </thead>
            <tbody>
              {live.map((app) => {
                const ipo = getIpo(app.ipoSlug);
                const profit = estimatedProfitPaise(app);
                return (
                  <tr key={app.id} className="border-t border-line">
                    <td className="px-4 py-3">
                      <Link to="/ipo/$slug" params={{ slug: app.ipoSlug }} className="font-medium">
                        {ipo?.name ?? app.ipoSlug}
                      </Link>
                      <p className="text-xs text-muted">{app.registrar}</p>
                    </td>
                    <td className="px-4 py-3 text-muted">
                      {app.source === "HISTORICAL" ? "Historical" : "Invest"}
                    </td>
                    <td className="px-4 py-3">
                      <StatusBadge status={jobState(app)} />
                    </td>
                    <td className="px-4 py-3 tabular-nums">{inr(plannedPaise(app) / 100)}</td>
                    <td className="px-4 py-3 tabular-nums text-muted">
                      {profit == null ? "Not available yet" : inr(profit / 100)}
                    </td>
                    <td className="px-4 py-3 text-right">
                      {app.voided ? (
                        <span className="text-xs text-subtle">Voided</span>
                      ) : (
                        <div className="flex justify-end gap-3">
                          <Link to="/allotment" className="text-xs text-muted hover:text-fg">
                            Check
                          </Link>
                          <button
                            type="button"
                            className="text-xs text-loss"
                            onClick={() => voidApplication(app.id)}
                          >
                            Void
                          </button>
                        </div>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {showHist ? (
        <form onSubmit={addHistorical} className="grid max-w-xl gap-3">
          <Panel>
            <h2 className="font-medium">Historical application</h2>
            <p className="mt-2 text-sm text-muted">
              Owner-entered fact from before this desk. Creates zero provider requests and zero
              allotment results.
            </p>
            <div className="mt-4 grid gap-3">
              <select
                className="h-11 rounded-md border border-line bg-raised px-3 text-sm"
                value={histSlug}
                onChange={(e) => setHistSlug(e.target.value)}
              >
                {IPOS.map((i) => (
                  <option key={i.slug} value={i.slug}>
                    {i.name}
                  </option>
                ))}
              </select>
              <Input
                type="number"
                min={1}
                value={histLots}
                onChange={(e) => setHistLots(Math.max(1, Number(e.target.value)))}
              />
              <Input type="date" value={histDate} onChange={(e) => setHistDate(e.target.value)} />
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={affirmed}
                  onChange={(e) => setAffirmed(e.target.checked)}
                />
                I affirm this is a historical record, not a new application.
              </label>
              {msg ? <p className="text-sm text-gain">{msg}</p> : null}
              <Button type="submit" variant="outline">
                Record historical
              </Button>
              <p className="text-xs text-subtle">
                Cost hint {inr(minInvest(getIpo(histSlug) ?? IPOS[0]) * histLots)} on{" "}
                {formatDate(histDate)}.
              </p>
            </div>
          </Panel>
        </form>
      ) : null}
    </div>
  );
}
