# Status: Fable 5.1 Finance Dashboard UI Refinement

- **Gate 1 — Repo scan and chart-data audit: DONE** (2026-09-05)
  - Current UI ships the calm Aether console in `apps/desktop/src/App.tsx` (5 views:
    Dashboard, Investments, Check Allotment, Members, Settings) + one `styles.css`.
  - Existing honest primitives: `FinanceBar`, `ProgressMeter`, `StatusRing`, `StatusBadge`.
  - Audit result: the current DTOs carry legitimate categorical and per-entity numeric
    breakdowns (per-IPO planned amounts, status buckets, final/pending counts, per-account
    applied vs estimated profit, friend `share_basis_points`, price bands, bidding dates,
    allocation ratios) — enough to make the UI chart-rich **without** violating the Aether
    "no invented series / no decorative charts" rule.
- **Gate 2 — Fable 5.1 prompt: READY** — see `01-fable51-prompt.md`.
  - One copy-paste prompt block, grounded in real files and real fields.
  - Hard constraints encoded: no new deps, CSS/SVG charts only, no Rust changes, no
    fabricated data, preserve test-asserted labels/roles, reduced-motion, accessibility,
    and the PRIVATE / LOCAL / AUDITED trust language.
- **Gate 3 — Execution: NOT STARTED**
  - The prompt is written to be run where Fable 5.1 is available (Claude Design model
    picker or a Fable-capable harness) with repo read access, output applied in-repo.
  - Local fallback available on request: implement the same chart set directly in this
    repo (no Fable needed) and validate with `npm run check`.
