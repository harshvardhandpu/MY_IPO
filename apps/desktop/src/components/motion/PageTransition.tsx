import type { ReactNode } from "react";

// Keyed remount on navigation gives every page a coordinated, subtle enter motion.

export function PageTransition({ pageKey, children }: { pageKey: string; children: ReactNode }) {
  return (
    <div key={pageKey} className="page-motion" data-page={pageKey}>
      {children}
    </div>
  );
}
