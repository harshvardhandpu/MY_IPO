import { useMemo, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PageHeader, Panel, SecurityStrip } from "@/components/page-header";
import { maskLast4, useLedger } from "@/lib/ledger";
import { cn, hashSeed } from "@/lib/utils";

export const Route = createFileRoute("/members")({ component: Members });

function Members() {
  const accounts = useLedger((s) => s.accounts);
  const addFriend = useLedger((s) => s.addFriend);
  const archiveAccount = useLedger((s) => s.archiveAccount);
  const setShare = useLedger((s) => s.setShare);
  const live = useMemo(() => accounts.filter((a) => !a.archived), [accounts]);
  const core = live.filter((a) => a.kind === "PRIMARY");
  const friends = live.filter((a) => a.kind === "FRIEND");
  const [label, setLabel] = useState("");
  const [last4, setLast4] = useState("");
  const [share, setSharePct] = useState("10");
  const [error, setError] = useState("");
  const [open, setOpen] = useState(false);

  function add(e: React.FormEvent) {
    e.preventDefault();
    const l4 = last4.trim().toUpperCase();
    if (!label.trim()) {
      setError("Give the friend book a label.");
      return;
    }
    if (!/^[A-Z0-9]{4}$/.test(l4)) {
      setError("Store only the last four PAN characters — never the full PAN.");
      return;
    }
    addFriend({
      label: label.trim(),
      last4: l4,
      shareBps: Math.round(Number(share || "0") * 100),
      eligible: Number(share) > 0,
    });
    setLabel("");
    setLast4("");
    setError("");
    setOpen(false);
  }

  return (
    <div className="grid max-w-3xl gap-6 pb-8">
      <PageHeader
        kicker="Roster"
        title="Members"
        body="Full PAN never lives on this desk. Only a four-character mask is stored, locally."
        actions={
          <Button onClick={() => setOpen((v) => !v)}>
            {open ? "Close form" : "Add friend account"}
          </Button>
        }
      />
      <SecurityStrip />

      <div className="grid grid-cols-2 gap-3">
        <Panel>
          <p className="text-xs text-muted">Core members</p>
          <p className="mt-2 font-display text-3xl tabular-nums">{core.length}</p>
        </Panel>
        <Panel>
          <p className="text-xs text-muted">Friend accounts</p>
          <p className="mt-2 font-display text-3xl tabular-nums">{friends.length}</p>
        </Panel>
      </div>

      <section className="grid gap-3">
        <p className="text-xs text-muted">Core members</p>
        {core.map((a) => (
          <AccountRow
            key={a.id}
            label={a.label}
            meta={`Primary · ${maskLast4(a.last4)}`}
            slug={a.id}
          />
        ))}
      </section>

      <section className="grid gap-3">
        <p className="text-xs text-muted">Friend accounts</p>
        {friends.length === 0 ? (
          <p className="rounded-xl bg-surface px-4 py-8 text-center text-sm text-muted">
            No friend accounts yet. Add one without exposing plaintext PAN.
          </p>
        ) : (
          friends.map((a) => (
            <article key={a.id} className="rounded-xl bg-surface p-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="flex items-center gap-3">
                  <Monogram slug={a.id} name={a.label} />
                  <div>
                    <p className="font-medium">{a.label}</p>
                    <p className="text-xs text-muted">
                      Friend · {maskLast4(a.last4)} · {a.shareBps / 100}% share
                    </p>
                  </div>
                </div>
                <Button variant="ghost" onClick={() => archiveAccount(a.id)}>
                  Archive
                </Button>
              </div>
              <label className="mt-4 flex items-center gap-3 text-sm text-muted">
                Profit share
                <Input
                  className="w-24"
                  type="number"
                  min={0}
                  max={50}
                  value={String(a.shareBps / 100)}
                  onChange={(e) => {
                    const pct = Number(e.target.value);
                    setShare(a.id, Math.round(pct * 100), pct > 0);
                  }}
                />
                %
              </label>
            </article>
          ))
        )}
      </section>

      {open ? (
        <form onSubmit={add} className="grid gap-3 rounded-xl bg-surface p-5">
          <h2 className="font-medium">Add friend account</h2>
          <Input value={label} onChange={(e) => setLabel(e.target.value)} placeholder="Label" />
          <Input
            value={last4}
            onChange={(e) => setLast4(e.target.value.toUpperCase())}
            placeholder="Last 4 of PAN"
            maxLength={4}
            autoComplete="off"
          />
          <Input
            value={share}
            onChange={(e) => setSharePct(e.target.value)}
            placeholder="Share %"
            type="number"
            min={0}
            max={50}
          />
          {error ? <p className="text-sm text-loss">{error}</p> : null}
          <Button type="submit">Add friend</Button>
        </form>
      ) : null}
    </div>
  );
}

function AccountRow({ label, meta, slug }: { label: string; meta: string; slug: string }) {
  return (
    <article className="flex items-center gap-3 rounded-xl bg-surface p-4">
      <Monogram slug={slug} name={label} />
      <div>
        <p className="font-medium">{label}</p>
        <p className="text-xs text-muted">{meta}</p>
      </div>
    </article>
  );
}

function Monogram({ slug, name }: { slug: string; name: string }) {
  const tones = ["bg-chip", "bg-heat", "bg-gain", "bg-fund", "bg-accent"] as const;
  return (
    <span
      className={cn(
        "flex size-11 items-center justify-center rounded-full text-xs font-medium text-bg",
        tones[hashSeed(slug) % tones.length],
      )}
    >
      {name.slice(0, 2).toUpperCase()}
    </span>
  );
}
