import { useMemo, useState } from "react";
import { Link, createFileRoute, useNavigate } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PageHeader, Panel, SecurityStrip } from "@/components/page-header";
import { IPOS, gmpPct, minInvest } from "@/lib/ipo-data";
import { providerForRegistrar, useLedger } from "@/lib/ledger";
import { previewRecommendation } from "@/lib/ranking";
import { cn, inr } from "@/lib/utils";

type InvestSearch = { slug?: string };

export const Route = createFileRoute("/invest")({
  validateSearch: (s: Record<string, unknown>): InvestSearch => ({
    slug: typeof s.slug === "string" ? s.slug : undefined,
  }),
  component: Invest,
});

function Invest() {
  const nav = useNavigate();
  const search = Route.useSearch();
  const accounts = useLedger((s) => s.accounts);
  const submitInvestment = useLedger((s) => s.submitInvestment);
  const active = useMemo(() => accounts.filter((a) => !a.archived), [accounts]);
  const open = IPOS.filter((i) => i.status === "open" || i.status === "upcoming");
  const initial = open.find((i) => i.slug === search.slug) ?? open[0] ?? IPOS[0];
  const [slug, setSlug] = useState(initial.slug);
  const [lots, setLots] = useState(1);
  const [capital, setCapital] = useState(60000);
  const [picked, setPicked] = useState<string[]>(active[0] ? [active[0].id] : []);
  const [preview, setPreview] = useState<ReturnType<typeof previewRecommendation> | null>(null);
  const [note, setNote] = useState("");

  const ipo = useMemo(() => IPOS.find((i) => i.slug === slug) ?? IPOS[0], [slug]);
  const each = minInvest(ipo) * lots;
  const total = each * picked.length;
  const over = total > capital;
  const provider = providerForRegistrar(ipo.registrar);

  function toggle(id: string) {
    setPicked((p) => (p.includes(id) ? p.filter((x) => x !== id) : [...p, id]));
    setPreview(null);
  }

  function check() {
    if (!picked.length) {
      setNote("Select at least one account.");
      return;
    }
    setNote("");
    setPreview(
      previewRecommendation({
        ipoSlug: slug,
        capital,
        selectedAccounts: picked.length,
        lots,
      }),
    );
  }

  function submit() {
    if (!preview) {
      setNote("Preview the recommendation before recording.");
      return;
    }
    submitInvestment({
      ipoSlug: slug,
      accountIds: picked,
      lots,
      source: "INVEST",
    });
    nav({ to: "/investments" });
  }

  return (
    <div className="grid gap-6 pb-8">
      <PageHeader
        kicker="Investment"
        title="Invest"
        body="CHECK previews a public-data ranking. SUBMIT records the human decision into the local ledger. Nothing is sent to a registrar here."
      />
      <SecurityStrip>
        Ranking uses only catalogue GMP and subscription. Owner algorithm is a placeholder.
      </SecurityStrip>

      <div className="grid gap-6 lg:grid-cols-[1.1fr_0.9fr]">
        <Panel>
          <h2 className="text-sm text-muted">Issue</h2>
          <div className="mt-4 flex gap-3 overflow-x-auto pb-1">
            {open.map((item) => (
              <button
                key={item.slug}
                type="button"
                onClick={() => {
                  setSlug(item.slug);
                  setPreview(null);
                }}
                className={cn(
                  "w-40 shrink-0 rounded-lg p-3 text-left",
                  slug === item.slug ? "bg-window" : "bg-raised",
                )}
              >
                <p className="truncate text-sm font-medium">{item.name}</p>
                <p className="mt-1 text-xs text-muted">
                  {item.board} · GMP {gmpPct(item).toFixed(0)}%
                </p>
                <p className="mt-2 text-xs tabular-nums text-subtle">{inr(minInvest(item))}</p>
              </button>
            ))}
          </div>

          <div className="mt-5 grid gap-4 sm:grid-cols-2">
            <label className="grid gap-2 text-sm">
              Daily capital
              <Input
                type="number"
                min={0}
                value={capital}
                onChange={(e) => {
                  setCapital(Number(e.target.value));
                  setPreview(null);
                }}
              />
            </label>
            <label className="grid gap-2 text-sm">
              Lots per account
              <Input
                type="number"
                min={1}
                value={lots}
                onChange={(e) => {
                  setLots(Math.max(1, Number(e.target.value)));
                  setPreview(null);
                }}
              />
            </label>
          </div>

          <p className="mt-4 text-sm text-muted">
            {inr(each)} each · {inr(total)} across {picked.length} account
            {picked.length === 1 ? "" : "s"}
            {over ? " · exceeds daily capital" : ""}
          </p>
          <p className="mt-1 text-xs text-subtle">
            {ipo.registrar} · {provider.label}
            {provider.auto ? " auto check later" : " human verification later"} · {ipo.lot} shares /
            lot
          </p>

          <fieldset className="mt-5 grid gap-2">
            <legend className="text-sm">Accounts</legend>
            {active.map((a) => (
              <label
                key={a.id}
                className="flex min-h-11 items-center gap-3 rounded-md bg-raised px-3 text-sm"
              >
                <input
                  type="checkbox"
                  checked={picked.includes(a.id)}
                  onChange={() => toggle(a.id)}
                />
                {a.label}
                <span className="text-xs text-subtle">
                  {a.kind === "PRIMARY" ? "Primary" : "Friend"}
                </span>
              </label>
            ))}
          </fieldset>
        </Panel>

        <div className="grid gap-4 self-start">
          {preview ? (
            <Panel className="bg-raised">
              <p className="text-xs text-muted">{preview.label}</p>
              <p className="mt-2 text-lg font-medium">
                {preview.skip ? "Skip" : `Apply ${preview.recommendedAccounts} account(s)`}
              </p>
              <p className="mt-2 text-sm text-muted">{preview.reason}</p>
              <p className="mt-2 text-xs text-subtle">
                Confidence {preview.confidence} · public catalogue only
              </p>
              {preview.skip ? (
                <p className="mt-3 text-sm text-warn">
                  Ranking suggests skip. You can still record if that is the owner decision.
                </p>
              ) : null}
            </Panel>
          ) : (
            <Panel>
              <p className="text-sm text-muted">
                Preview recommendation first. CHECK never records capital or talks to a registrar.
              </p>
            </Panel>
          )}

          {over ? (
            <p className="text-sm text-warn">
              Planned {inr(total)} is above daily capital {inr(capital)}. Arithmetic warning only —
              submit remains the owner’s call.
            </p>
          ) : null}
          {note ? <p className="text-sm text-loss">{note}</p> : null}

          <div className="grid gap-2">
            <Button variant="outline" onClick={check}>
              Preview recommendation
            </Button>
            <Button onClick={submit} disabled={!preview}>
              Record investment
            </Button>
          </div>
          <p className="text-xs text-subtle">
            Check allotment is a separate, purpose-scoped step after the session is recorded.
          </p>
          <Link to="/investments" className="text-sm text-muted hover:text-fg">
            Open the investment ledger
          </Link>
        </div>
      </div>
    </div>
  );
}
