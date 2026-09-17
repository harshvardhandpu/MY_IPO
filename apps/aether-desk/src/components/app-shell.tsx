import { useMemo, useState, type ReactNode } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import {
  CalendarDays,
  Bell,
  Gift,
  LayoutGrid,
  List,
  Plus,
  ScrollText,
  Search,
  Users,
  Wallet,
  X,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { IPOS } from "@/lib/ipo-data";
import { WIDGETS, useWidgets } from "@/lib/widgets";
import { Button } from "@/components/ui/button";

export function AppShell({ children }: { children: ReactNode }) {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const [searchOpen, setSearchOpen] = useState(false);
  const [widgetOpen, setWidgetOpen] = useState(false);
  const [query, setQuery] = useState("");
  const widgets = useWidgets();

  const hits = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return IPOS.slice(0, 8);
    return IPOS.filter(
      (i) =>
        i.name.toLowerCase().includes(q) ||
        i.ticker.toLowerCase().includes(q) ||
        i.sector.toLowerCase().includes(q),
    ).slice(0, 12);
  }, [query]);

  return (
    <div className="relative flex min-h-dvh w-full flex-col bg-window text-fg">
      <header className="flex items-center justify-between gap-3 px-4 py-3 sm:px-6">
        <div className="flex items-center gap-3">
          <span className="flex size-11 items-center justify-center rounded-full bg-raised text-sm font-medium">
            HV
          </span>
          <div>
            <p className="text-sm font-medium leading-tight">Harshvardhan</p>
            <p className="text-xs text-muted">Investor · Synthetic</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button asChild className="hidden sm:inline-flex">
            <Link to="/invest">Invest</Link>
          </Button>
          <Button
            type="button"
            variant="outline"
            className="hidden sm:inline-flex"
            onClick={() => setWidgetOpen(true)}
          >
            Add widget
            <Plus className="size-4" />
          </Button>
          <IconBtn label="Alerts">
            <Gift className="size-4" />
          </IconBtn>
          <IconBtn label="Notifications">
            <Bell className="size-4" />
          </IconBtn>
          <IconBtn label="Search" onClick={() => setSearchOpen(true)}>
            <Search className="size-4" />
          </IconBtn>
        </div>
      </header>

      <main className="flex-1 overflow-y-auto px-4 pb-32 sm:px-6">{children}</main>

      <nav className="pointer-events-none absolute inset-x-0 bottom-4 z-20 flex justify-center px-4">
        <div className="pointer-events-auto flex items-center gap-1 rounded-full bg-raised/95 px-2 py-2 shadow-window">
          <DockLink to="/" active={pathname === "/"} label="Desk">
            <LayoutGrid className="size-4" />
          </DockLink>
          <DockLink to="/members" active={pathname.startsWith("/members")} label="Members">
            <Users className="size-4" />
          </DockLink>
          <DockLink to="/books" active={pathname.startsWith("/books")} label="Books">
            <List className="size-4" />
          </DockLink>
          <Link
            to="/invest"
            aria-label="Invest"
            className="mx-1 flex size-12 items-center justify-center rounded-full bg-accent text-accent-fg"
          >
            <Plus className="size-5" />
          </Link>
          <DockLink
            to="/allotment"
            active={pathname.startsWith("/allotment")}
            label="Allotment"
          >
            <Wallet className="size-4" />
          </DockLink>
          <DockLink
            to="/investments"
            active={pathname.startsWith("/investments")}
            label="Ledger"
          >
            <ScrollText className="size-4" />
          </DockLink>
          <span className="hidden sm:inline-flex">
            <DockLink
              to="/calendar"
              active={pathname.startsWith("/calendar")}
              label="Calendar"
            >
              <CalendarDays className="size-4" />
            </DockLink>
          </span>
        </div>
      </nav>

      {searchOpen ? (
        <Overlay title="Search the book" onClose={() => setSearchOpen(false)}>
          <input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Name, ticker, sector"
            className="h-11 w-full rounded-md border border-line bg-raised px-3 text-sm outline-none placeholder:text-subtle focus-visible:ring-2 focus-visible:ring-accent/40"
          />
          <ul className="mt-4 grid gap-1">
            {hits.map((ipo) => (
              <li key={ipo.slug}>
                <Link
                  to="/ipo/$slug"
                  params={{ slug: ipo.slug }}
                  onClick={() => setSearchOpen(false)}
                  className="flex min-h-11 items-center justify-between rounded-md px-3 hover:bg-raised"
                >
                  <span>
                    <span className="block text-sm">{ipo.name}</span>
                    <span className="text-xs text-muted">
                      {ipo.ticker} · {ipo.status}
                    </span>
                  </span>
                  <span className="text-xs text-muted">{ipo.board}</span>
                </Link>
              </li>
            ))}
          </ul>
        </Overlay>
      ) : null}

      {widgetOpen ? (
        <Overlay title="Widgets" onClose={() => setWidgetOpen(false)}>
          <p className="mb-4 text-sm text-muted">Show or hide cards on the desk.</p>
          <ul className="grid gap-2">
            {WIDGETS.map((w) => {
              const on = widgets.visible(w.id);
              return (
                <li key={w.id}>
                  <button
                    type="button"
                    onClick={() => widgets.toggle(w.id)}
                    className="flex h-11 w-full items-center justify-between rounded-md bg-raised px-4 text-sm"
                  >
                    {w.label}
                    <span className={on ? "text-gain" : "text-subtle"}>{on ? "On" : "Off"}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </Overlay>
      ) : null}
    </div>
  );
}

function IconBtn({
  label,
  children,
  onClick,
}: {
  label: string;
  children: ReactNode;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className="flex size-11 items-center justify-center rounded-full bg-raised text-fg"
    >
      {children}
    </button>
  );
}

function DockLink({
  to,
  active,
  label,
  children,
}: {
  to: "/" | "/books" | "/allotment" | "/calendar" | "/tools" | "/members" | "/invest" | "/investments";
  active: boolean;
  label: string;
  children: ReactNode;
}) {
  return (
    <Link
      to={to}
      aria-label={label}
      className={cn(
        "flex size-11 items-center justify-center rounded-full",
        active ? "bg-window text-fg" : "text-muted hover:text-fg",
      )}
    >
      {children}
    </Link>
  );
}

function Overlay({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-end justify-center bg-bg/70 p-3 sm:items-center">
      <div className="max-h-[80dvh] w-full max-w-lg overflow-y-auto rounded-xl bg-surface p-5 text-fg">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-medium">{title}</h2>
          <button
            type="button"
            aria-label="Close"
            onClick={onClose}
            className="flex size-11 items-center justify-center rounded-full bg-raised"
          >
            <X className="size-4" />
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
