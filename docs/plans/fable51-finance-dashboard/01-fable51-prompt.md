# Fable 5.1 Prompt — "Finance Dashboard" UI Refinement

> **What this is:** a copy-paste design prompt for **Fable 5.1** (Anthropic's UI/frontend
> design model — pick it in the Claude Design model selector, or run it in any harness that
> has Fable 5.1 access) that turns Sanket IPO's calm Aether console into a **chart-first
> private finance dashboard**.
>
> It is grounded in the real repo state: every chart below renders a field that already
> exists in the DTO loaded on that screen. **No fabricated series, no invented time lines.**
> The prompt is written to be executed by an agent with repo access (it can read
> `App.tsx`, `styles.css`, and the test files before editing).
>
> Suggested Fable effort: **high**. Deliverable: code applied in-repo, not a static mockup.

---

## THE PROMPT — copy everything inside this block

```text
You are a senior product designer and frontend engineer refining the UI of "Sanket IPO".

PROJECT CONTEXT
- Sanket IPO is a private, local-first Windows/Linux desktop operating system for a trusted
  Indian IPO investment group. Tauri 2 + React 19 + TypeScript + Vite frontend; Rust backend
  (crates/) is OUT OF SCOPE. Repo: /home/user/MY_IPO, frontend at apps/desktop/.
- Trust boundary is sacred: member identities, allocations, accounting and audit never leave
  the device and never reach an external AI. Public IPO intelligence is the only domain that
  may touch external services, through an allowlisted gateway. The UI must keep feeling
  PRIVATE / LOCAL / AUDITED, never like a consumer fintech marketing page.
- Current frontend: one large apps/desktop/src/App.tsx (~3,200 lines) holds all views;
  apps/desktop/src/styles.css holds one semantic token set; apps/desktop/src/main.tsx mounts
  it. There are already small honest chart primitives: FinanceBar (horizontal ratio bar),
  ProgressMeter, StatusRing (conic-gradient donut), StatusBadge. Tests live beside App.tsx
  (App.test.tsx, App.flows.test.tsx) and assert exact labels/strings.
- READ App.tsx, styles.css, index.html, package.json and the two test files BEFORE editing so
  you preserve every string, role and flow the tests depend on.

MISSION
- Refine the current calm console into a dense, trustworthy PRIVATE FINANCE DASHBOARD:
  chart-first and data-dense (density ~8–9 on a 1–10 dial), restrained motion (~2), semantic
  color only, aligned numbers everywhere. The user explicitly wants to SEE lots of bars and
  charts: bar charts, stacked bars, donuts, score bars, timeline bars, diverging profit bars.
- HARD HONESTY RULE: every chart is a pure rendering of a numeric field that already exists
  in the data that screen loads. Never invent, interpolate, extrapolate, mock, or seed fake
  series. No decorative charts, no fake growth curves, no synthetic "portfolio over time"
  unless a real series exists (it does not today). Empty, loading, stale and error states
  stay explicit on every panel.

DESIGN LANGUAGE TO KEEP (existing tokens in styles.css :root — do not replace, extend if
needed and keep them in styles.css):
- Canvas #f1f4f1, sidebar #fbfcfa, surface #ffffff, surface-subtle #f6f8f6, borders
  #dce4de / strong #b9c8bd, text #18221c, muted #59685e, faint #77847a.
- Accent/action green #247352 (hover #1b5c41, soft #eaf3ed) — primary actions and focus.
  Positive/emerald #177a54, warning/amber #93620e, danger/vermilion #b33f39,
  unknown/violet-gray #645c92 — each with its "-soft" background. Color NEVER carries meaning
  alone: StatusBadge keeps its glyph + text + tone grammar (✓ − ? ! ○ ↻ × M).
- Radii: controls 6px, panels 10px. Shadows reserved for dialogs/menus; normal hierarchy is
  hairline borders. Typography: existing system-variable stack; 28px page title, 16px section,
  13px body, 11px uppercase labels; tabular numerals for all money/counts/IDs/dates/rankings.
  Format money only through the existing formatRupees (en-IN, ₹, integer rupees).
- Motion: subtle 120–240ms ease for hover/press/progress only, gated behind
  prefers-reduced-motion (the codebase already has this pattern — reuse it).
- Visual character: a private capital ledger + registrar operations console — compact,
  aligned, quiet chrome, loud only where money moves or an action records something.

CHART SET TO IMPLEMENT (each item: what it is, and the exact source field)
DASHBOARD — loads DashboardData (total_planned_paise, submitted_session_count, member_count,
friend_count, profit_paise) plus AllotmentCandidate[] "activity" (ipo_name,
planned_amount_paise, overall_job_state, pending_count, final_count):
1. KPI strip: keep the four summary cards (Total invested, Realized profit, Applications,
   Accounts). Under each value add a small real contextual ratio bar, e.g. profit/capital,
   checks finalized vs total, core vs friend accounts. All numbers real and labeled.
2. "Capital overview" panel → becomes a two-part visual: (a) a diverging horizontal bar of
   Active capital vs Realized profit (both real); (b) a VERTICAL COLUMN CHART "Deployed
   capital by IPO" from activity[].planned_amount_paise, sorted descending, value labels on
   each column, truncated IPO names below. If there are more than ~8 rows, collapse the tail
   into one real column labeled "Other (N)" with the true summed value.
3. "Allotment status" panel: keep the donut over overall_job_state buckets (real), then add a
   per-application horizontal STACKED BAR of final vs pending accounts from final_count +
   pending_count with a right-aligned % of completion. Tone follows the job state.
4. "Portfolio composition": one horizontal stacked 100% bar of core vs friend accounts
   (member_count / friend_count) with counts in the segments and exact values in the legend.
5. Keep the "Current activity" data table as the dense operational source of truth (tables
   stay; do not replace records with cards). Keep Quick actions and the CHECK → SUBMIT
   decision-boundary strip intact and restyled to match.

INVESTMENTS (Invest view) — loads open + upcoming IpoCatalogue items, selected
IpoCatalogItem details, CheckResponse recommendation (RankedIpo[]: ranking, score,
recommended_account_count, recommended_allocation_ratio_bp, skip, reason), and the composer's
own computed totals:
1. Replace the two plain card columns (OPEN / UPCOMING) with a compact pipeline list where
   each IPO row carries a horizontal PHASE TIMELINE BAR from bidding_start_date →
   bidding_end_date with a "today" marker and a phase color from item.status, plus price band
   and issue-size hints. Dates are real fields; when null show "TBA", never guess.
2. IPO details drawer: add a PRICE BAND BAR plotting minimum_price_paise →
   planning_price_paise/cut_off → maximum_price_paise on one axis with labeled endpoints.
   Add a subscription meter only when item.total_subscription parses as "N.Nx" — otherwise
   render the raw string as text with no bar.
3. Recommendation review panel: ranked IPOs become SCORE BARS (score 0–100 real) with rank,
   and a single 100% ALLOCATION-SHARE STACKED BAR across non-skipped IPOs built from
   recommended_allocation_ratio_bp; skipped rows stay visually muted with their reason.
4. Composer aside: when plans are complete, show a real stacked bar of planned capital per
   included IPO (computed per-IPO amount × selected account count) and a selected-accounts
   count with core/friend split.

CHECK ALLOTMENT (Allotment view) — loads AllotmentCandidate[] queue and the per-job
AllotmentJobReport (rows[]: account_kind, status, allotted_lots, allotted_shares,
application_amount_paise, estimated_profit_paise, profit_basis):
1. Candidate queue: each selectable row keeps its radio + progress; upgrade the progress to a
   mini final/pending STACKED BAR with a completion % right-aligned.
2. Report card: keep the job status strip and ProgressMeter; add (a) a horizontal STACKED BAR
   of account count by status from rows[].status using the existing status tones, and
   (b) per-account DIVERGING BARS of applied capital (application_amount_paise, accent)
   against estimated profit (estimated_profit_paise, positive when >0 / danger when <0) with
   the profit_basis label visible; allotted lots/shares stay as aligned numbers.
3. Manual-result and verification flows keep their current layout, restyled only.

MEMBERS — member_count/friend_count plus FriendRow.share_basis_points:
1. Keep the count grid; add one horizontal stacked bar of core vs friend totals.
2. Friend rows get a tiny real allocation-share bar from share_basis_points (basis points →
   %) so the member list reads like an allocation register.

SETTINGS — keep calm and dense; no charts needed beyond the existing connection/status pills.
Do not force charts where the view has no chartable breakdown; honesty first, density second.

IMPLEMENTATION RULES (HARD CONSTRAINTS)
1. No new npm dependencies. No chart library, no three.js/webgl/canvas — implement
   columns/bars/donuts/timelines with CSS, divs, conic-gradient and inline SVG only.
2. Do NOT touch apps/desktop/src-tauri or anything under crates/. No new invoke commands, no
   schema/domain/security/provider changes, no new navigation destinations.
3. Never render a full PAN; only the existing masked values. No new network calls from the
   frontend. Keep the local/privacy status language in the top bar and sidebar footer.
4. Preserve every accessible name, role and string the tests assert (read the tests): e.g.
   nav labels Dashboard/Investments/Check Allotment/Members/Settings, aria-label "Quick
   actions", buttons "Invest", "Check Allotment", "CHECK"/"SUBMIT", "Saved locally",
   "Pending sync", workflow headings such as the recommendation disclaimer. If a copy change
   is genuinely necessary, update the affected test in the SAME change and report it.
5. Structure: extract new chart components under apps/desktop/src/components/charts/
   (e.g. ColumnChart, StackedBar, DivergingBar, Donut, TimelineBar, ScoreBar, RatioBar) as
   small pure typed React components with explicit data props {label, value, format, tone}.
   They must render correctly under jsdom (no requestAnimationFrame gating the final value,
   no layout measurement); every chart exposes its numbers as DOM text and a concise
   role="img" + aria-label summary, or the correct ARIA role (progressbar, list, img) — never
   a silent visual.
6. Accessibility: full keyboard reachability, visible focus ring (existing accent outline),
   prefers-reduced-motion renders settled values with no animation, color is never the only
   signal, AA contrast, tabular numerals, and preserved empty/loading/stale/error states.
7. Layout: keep the 224px sidebar + main grid; the app targets ~1280–1440px desktop and must
   not scroll horizontally at ≥1024px. Charts scale to their panel, do not force widths.
8. Reuse the existing glyph vocabulary and system font stack; do not add an icon package.

VERIFY BEFORE FINISHING (run in apps/desktop or repo root as appropriate)
- npm run typecheck (tsc -b) passes.
- npm test passes — both App.test.tsx and App.flows.test.tsx; fix regressions, do not skip.
- npm run lint and npm run secrets pass if available.
- Visually walk every view's success, empty, loading and error states in dev and note what
  you checked.
- Report: per-view chart list with source fields, files touched, tokens added/changed, and
  any test updates, plus anything intentionally left out and why (data-honesty reasons).

OUTPUT SHAPE
- Implement directly in the repo: surgical edits to the view render sections in App.tsx,
  additions to styles.css, and new files under src/components/charts/.
- Finish with a short handoff summary in the same style as the verification report.
```

---

## Usage notes

### How to run it

1. **Where Fable 5.1 lives.** This repo has no Fable integration — Fable 5.1 is a model
   (run inside Claude Design's model picker, or any agent harness with Fable 5.1 access).
   Give the harness **read access to the repo** and paste the block above verbatim; the
   prompt expects it to read `App.tsx`, `styles.css`, and the tests before editing.
2. **Landing the output.** Prefer Fable output as *code changes to this repo* (its editor
   mode), then run `npm run check` locally before committing. If the output is a static
   design instead, hand it to a coding agent alongside the same brief.
3. **Scope lever.** The prompt targets all 5 views. For a first pass, truncate the
   "CHART SET TO IMPLEMENT" section to just **DASHBOARD** (everything else still works).

### Why this chart set and not more

The earlier Aether plan shipped *zero* charts on purpose: "current data does not expose a
legitimate time series." That rule is kept — the brief refuses time series, growth curves,
and simulated returns. What changed is that the current DTOs *do* carry legitimate
categorical and per-entity numeric breakdowns, which is exactly what bar/stacked/donut/
score/timeline charts render:

| Chart (view) | Source field(s) already on the screen |
|---|---|
| Deployed capital by IPO (Dashboard) | `activity[].planned_amount_paise` |
| Status donut + final/pending stacks (Dashboard, Allotment) | `overall_job_state`, `final_count`, `pending_count`, `rows[].status` |
| Capital vs profit divergence (Dashboard) | `total_planned_paise`, `profit_paise` |
| Account composition (Dashboard, Members) | `member_count`, `friend_count`, `FriendRow.share_basis_points` |
| Phase timeline + price band (Invest) | `bidding_start/end_date`, `minimum/maximum/planning_price_paise`, `status` |
| Recommendation score + allocation share (Invest) | `RankedIpo.score`, `recommended_allocation_ratio_bp` |
| Applied vs estimated profit (Allotment) | `rows[].application_amount_paise`, `rows[].estimated_profit_paise` |

### Iteration ladder (paste after the first pass)

- "Keep everything, but push density higher: shrink panel gutters to a 4px rhythm, tighten
  the KPI strip, and add column charts to the Allotment report card."
- "Restyle the chart ink: hairline grid lines, lighter ticks, value labels in text-faint
  #77847a, hover highlight via accent-soft #eaf3ed only under no-preference reduced motion."
- "Add a right-aligned mini donut of overall statuses to the Dashboard's activity table
  header — data already loaded, keep the table itself untouched."
- "Audit my implementation for honesty: flag any chart that is not a pure render of the DTO
  fields listed, any invented series, and any string the tests assert that you changed."

### Acceptance checklist (applies whether Fable or a local agent did the work)

- [ ] `npm run typecheck` and `npm test` pass in `apps/desktop`
- [ ] No new dependencies, no Rust/crate/`src-tauri` changes, no new invoke commands
- [ ] Every chart traces to a real field rendered on that screen (see table above)
- [ ] No chart animates under `prefers-reduced-motion`; every chart has text/ARIA equivalents
- [ ] Full PANs never render; top-bar privacy and sync language unchanged
- [ ] Empty / loading / stale / error states still present on every charted panel
