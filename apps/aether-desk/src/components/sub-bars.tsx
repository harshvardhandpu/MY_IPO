import type { Subscription } from "@/lib/ipo-data";

const rows: { key: keyof Subscription; label: string }[] = [
  { key: "qib", label: "QIB" },
  { key: "nii", label: "NII" },
  { key: "retail", label: "Retail" },
  { key: "total", label: "Total" },
];

export function SubBars({ sub }: { sub: Subscription }) {
  const peak = Math.max(sub.qib, sub.nii, sub.retail, sub.total, 1);
  return (
    <div className="grid gap-2">
      {rows.map((r) => {
        const v = sub[r.key] ?? 0;
        return (
          <div key={r.key} className="grid grid-cols-[4.5rem_1fr_3.2rem] items-center gap-2">
            <span className="text-xs text-muted">{r.label}</span>
            <div className="h-1.5 overflow-hidden rounded-full bg-raised">
              <div
                className="h-full rounded-full bg-accent"
                style={{ width: `${Math.min(100, (v / peak) * 100)}%` }}
              />
            </div>
            <span className="text-right font-mono text-xs tabular-nums text-fg">
              {v.toFixed(2)}x
            </span>
          </div>
        );
      })}
    </div>
  );
}
