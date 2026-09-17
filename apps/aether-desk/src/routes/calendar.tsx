import { Link, createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@/components/page-header";
import { IPOS } from "@/lib/ipo-data";
import { formatDate } from "@/lib/utils";

export const Route = createFileRoute("/calendar")({ component: CalendarPage });

const YEAR = 2026;
const MONTH = 8; // September 0-index

function CalendarPage() {
  const first = new Date(YEAR, MONTH, 1);
  const startPad = first.getDay();
  const days = new Date(YEAR, MONTH + 1, 0).getDate();
  const cells: (number | null)[] = [
    ...Array.from({ length: startPad }, () => null),
    ...Array.from({ length: days }, (_, i) => i + 1),
  ];
  while (cells.length % 7) cells.push(null);

  const byDay = new Map<string, typeof IPOS>();
  for (const ipo of IPOS) {
    for (const key of ["open", "close", "allotment", "listing"] as const) {
      const iso = ipo[key];
      const list = byDay.get(iso) ?? [];
      list.push(ipo);
      byDay.set(iso, list);
    }
  }

  const events = IPOS.flatMap((ipo) =>
    (
      [
        ["Opens", ipo.open],
        ["Closes", ipo.close],
        ["Allotment", ipo.allotment],
        ["Lists", ipo.listing],
      ] as const
    ).map(([kind, iso]) => ({ ipo, kind, iso })),
  ).sort((a, b) => a.iso.localeCompare(b.iso));

  return (
    <div className="grid gap-8 pb-8">
      <PageHeader
        kicker="September 2026"
        title="Issue calendar"
        body="Open, close, allotment, and listing dates from the public catalogue."
      />
      <div className="hidden overflow-hidden rounded-xl bg-surface md:block">
        <div className="grid grid-cols-7 bg-raised text-center text-xs uppercase tracking-wider text-subtle">
          {"Sun Mon Tue Wed Thu Fri Sat".split(" ").map((d) => (
            <div key={d} className="px-2 py-3">
              {d}
            </div>
          ))}
        </div>
        <div className="grid grid-cols-7">
          {cells.map((d, i) => {
            const iso = d ? `2026-09-${String(d).padStart(2, "0")}` : "";
            const hits = iso ? (byDay.get(iso) ?? []) : [];
            const unique = [...new Map(hits.map((h) => [h.slug, h])).values()];
            return (
              <div
                key={i}
                className="min-h-24 border-t border-l border-line p-2 first:border-l-0"
              >
                {d ? (
                  <>
                    <p className="text-xs text-muted">{d}</p>
                    <div className="mt-1 grid gap-1">
                      {unique.slice(0, 3).map((ipo) => (
                        <Link
                          key={ipo.slug}
                          to="/ipo/$slug"
                          params={{ slug: ipo.slug }}
                          className="truncate text-xs text-fg hover:text-accent"
                        >
                          {ipo.name}
                        </Link>
                      ))}
                      {unique.length > 3 ? (
                        <p className="text-xs text-subtle">+{unique.length - 3}</p>
                      ) : null}
                    </div>
                  </>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
      <ol className="grid gap-2">
        {events.map((e) => (
          <li
            key={`${e.ipo.slug}-${e.kind}-${e.iso}`}
            className="flex flex-wrap items-center justify-between gap-2 rounded-lg bg-surface px-4 py-3"
          >
            <div>
              <Link
                to="/ipo/$slug"
                params={{ slug: e.ipo.slug }}
                className="font-medium hover:text-accent"
              >
                {e.ipo.name}
              </Link>
              <p className="text-xs text-muted">
                {e.kind} · {e.ipo.board}
              </p>
            </div>
            <p className="font-mono text-sm tabular-nums text-muted">{formatDate(e.iso)}</p>
          </li>
        ))}
      </ol>
    </div>
  );
}
