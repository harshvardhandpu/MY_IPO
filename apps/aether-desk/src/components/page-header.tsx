import type { ReactNode } from "react";

export function PageHeader({
  kicker,
  title,
  body,
  actions,
}: {
  kicker?: string;
  title: string;
  body?: string;
  actions?: ReactNode;
}) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-4">
      <div className="max-w-2xl">
        {kicker ? <p className="text-xs text-muted">{kicker}</p> : null}
        <h1 className="mt-1 font-display text-3xl font-medium tracking-tight">{title}</h1>
        {body ? <p className="mt-2 text-sm text-muted">{body}</p> : null}
      </div>
      {actions ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
    </header>
  );
}

export function SecurityStrip({ children }: { children?: ReactNode }) {
  return (
    <p className="rounded-xl bg-raised px-4 py-3 text-sm">
      <span className="font-medium text-warn">DEVELOPMENT_SYNTHETIC</span>
      <span className="ml-2 text-muted">
        {children ??
          "Real PAN lookup is not authorized. Only a four-character mask is stored on this desk."}
      </span>
    </p>
  );
}

export function Panel({
  className = "",
  children,
}: {
  className?: string;
  children: ReactNode;
}) {
  return <section className={`rounded-xl bg-surface p-5 ${className}`}>{children}</section>;
}
