# Program Design: Aether UI Redesign

## Files

- `apps/desktop/src/App.tsx` - recompose existing views, add the shared status presentation helper, render compact navigation, and reuse safe read DTOs. Command names, request bodies, validation, and secure inputs remain unchanged.
- `apps/desktop/src/styles.css` - replace the existing visual layer with semantic light/dark tokens and shared desktop component styles.
- `apps/desktop/src/App.test.tsx` - preserve shell-level behavior and add navigation/status accessibility assertions.
- `apps/desktop/src/App.flows.test.tsx` - preserve secure onboarding, CHECK/SUBMIT, historical entry, and provider-independent allotment behavior while asserting the redesigned labels and provenance distinctions.
- `docs/plans/aether-ui-redesign/*` - approved design decisions, mockup, architecture, and slice state.

No new production file or npm dependency is required.

## Types and signatures

```ts
type View = "dashboard" | "members" | "invest" | "allotment";

type StatusTone =
  | "positive"
  | "neutral"
  | "info"
  | "warning"
  | "danger"
  | "unknown";

interface StatusPresentation {
  label: string;
  glyph: string;
  tone: StatusTone;
}

function statusPresentation(status: string): StatusPresentation;

function StatusBadge({ status }: { status: string }): React.ReactNode;

function DashboardView({
  bridge,
  dashboard,
  onInvest,
  onAllotment,
}: {
  bridge: CommandBridge;
  dashboard: DashboardData;
  onInvest: () => void;
  onAllotment: () => void;
}): React.ReactNode;
```

`statusPresentation` replaces the narrower `allotmentStatusLabel` mapping so every candidate, job, and account result routes through one semantic source. Unknown values still receive a readable normalized label and the unknown tone.

No command bridge, request, response, domain, or persistence signature changes.

## View composition

### App shell

- Compact 224px sidebar at wide desktop sizes.
- Four active destinations only, each with a consistent text glyph and label.
- Current view receives `aria-current="page"`.
- Sidebar footer states that visible identity is masked. It does not claim provider or OS-keyring state that has not been loaded.
- At narrow desktop widths, the sidebar becomes a compact top rail; content remains fully keyboard reachable.

### Dashboard

- Compact title and two prominent actions.
- One continuous summary strip for legitimate dashboard values instead of four oversized cards.
- Non-blocking `list_allotment_candidates` read for a current-activity table.
- Honest empty and read-error states do not affect the main boot state.
- Current activity shows IPO, registrar, status, amount, and last checked only when present in the existing DTO.
- A permanent decision-boundary row distinguishes recommendation preview from recorded investment.
- No charts because the current DTO has no trustworthy series or category breakdown.

### Onboarding

- Split secure-introduction and form workspace.
- Existing field order and native controls remain unchanged.
- PAN and UPI are grouped as protected identity fields with existing secure submission behavior.
- Consent remains required and retains its current request field.
- The submitted full PAN is never rendered; the existing masked response remains the only post-save value.

### Members

- Core owner and friend accounts render as compact operational rows.
- Masked PAN, role, share eligibility, and archive action remain visible.
- Friend creation stays behind the existing progressive disclosure.
- No investment/profit values are invented because member DTOs do not expose them.

### Investments

- Current and historical modes share one page title and an explicit two-option add switch.
- Current mode keeps daily cap, IPO rows, account selection, recommendation result, and sticky action rail.
- CHECK receives a preview label and outlined treatment.
- SUBMIT receives a record label and filled treatment, and stays disabled until recommendation exists.
- Historical mode keeps all existing fields and native date input.
- Historical source and unverified result state are presented as durable provenance, not development metadata.

### Check Allotment

- Two-pane console: candidate queue and selected operation/report.
- Candidate rows show registrar, amount, account progress, provider health, last checked, and shared status badge.
- Report header shows IPO, registrar, overall status, and checked timestamp.
- Account rows show kind, masked PAN, application amount, status, quantity, source, provenance, profit, and verification action.
- Automated and manual results receive explicit source labels.
- Human verification stays a normal action path with the official URL.
- Manual result and profit basis use native details disclosure to avoid permanently expanding dense rows.

## Call stack

### Dashboard current activity

1. `App` renders `DashboardView` after existing boot reads succeed.
2. `DashboardView.useEffect` invokes `list_allotment_candidates`.
3. Success stores safe rows locally.
4. Failure stores a local read error and renders a contextual empty state.
5. No retry loop and no mutation.

### Status rendering

1. Candidate, job, or account provides a status string.
2. `StatusBadge` calls `statusPresentation`.
3. The component renders a hidden or visible glyph plus exact label and tone class.
4. CSS maps the tone to semantic foreground/background/border tokens.

### Existing mutations

No call-stack change. Onboarding, friend creation/archive, CHECK, SUBMIT, historical save, allotment start/cancel/poll, manual result, and profit estimate continue to call their existing handlers and commands.

## Test plan

- `renders only implemented navigation destinations and marks the active page` - proves no fabricated product areas and accessible navigation state.
- `renders dashboard activity from the existing safe candidate DTO` - proves the UI does not require a backend expansion.
- `distinguishes unknown, not found, and not allotted without color alone` - asserts three distinct visible labels/glyphs.
- Existing `shows onboarding on an empty vault and sends identity only to the onboarding command` - remains unchanged in security meaning and still proves full PAN is absent after save.
- Existing `runs CHECK with approved fields, labels DEV output, then submits the reviewed session` - retains exact command payloads while asserting preview/record language.
- Existing `records an owner-affirmed historical application through the native form` - retains source, unknown date behavior, and exact command payload.
- Existing `presents one provider-independent allotment flow with mixed account progress` - retains provider behavior and asserts automated/manual provenance presentation.
- Frontend `typecheck`, `test`, and `build` remain mandatory after each slice.
- Final static boundary check rejects any changed file under `crates/` or `apps/desktop/src-tauri/` relative to `70a57bc`.

## Least confident decisions

1. The dashboard candidate read adds useful current activity but is intentionally non-blocking and has no automatic retry.
2. Native text glyphs avoid an icon dependency. Labels carry meaning; glyphs are only redundant non-color indicators.
3. Member investment/profit summaries are omitted because the safe member DTO does not expose them. Adding fake or recomputed values would violate the UI-only boundary.
4. Charts are omitted until a real time-series or grouped-performance DTO exists.
5. The redesign uses system light/dark preference rather than adding a Settings screen or theme persistence that the product does not currently have.
