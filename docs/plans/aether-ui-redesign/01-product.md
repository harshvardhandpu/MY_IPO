# Product: Aether UI Redesign

## Problem

Sanket IPO is functionally trustworthy but visually reads as an oversized cream-and-green web dashboard. Important financial actions, provenance, provider state, and the difference between recommendation and execution compete for attention instead of forming a calm operating hierarchy.

## Success metric

A keyboard user can reach either primary action from Dashboard and distinguish CHECK from SUBMIT, UNKNOWN from NOT ALLOTTED, and automated from manual provenance without opening explanatory documentation. Measure with the existing frontend interaction tests plus a visual and keyboard walkthrough of every active view.

## Announcement

Sanket IPO now presents the private investment ledger as a compact desktop workspace. Investment planning, historical entry, registrar checks, and account provenance share one restrained visual language. Dense records use tables and operational rows instead of oversized cards. Identity stays masked, security state stays visible, and recommendation actions remain visibly separate from recorded financial actions.

## Aether design read

A desktop financial operations workspace for an owner-operator, with a cold-luxury trust language. It should feel closer to a private ledger and registrar console than a consumer banking site or generic analytics dashboard.

- **Variance:** 4 - structured with selective asymmetry, never decorative chaos.
- **Motion:** 2 - press, disclosure, progress, and success feedback only.
- **Density:** 8 - compact rows, aligned numbers, restrained surfaces.
- **Theme:** system-aware light and dark through one semantic token set.
- **No charts in the first redesign:** current data does not expose a legitimate time series or category breakdown.

## Proposed design system

### Typography

- Native variable sans stack for labels and prose.
- Tabular numeric styling for currency, counts, IDs, dates, and rankings.
- Page title: 28px/34px, 650 weight.
- Section title: 16px/22px, 650 weight.
- Body: 13px/20px.
- Label and table header: 11px/16px, 650 weight, limited uppercase.

### Spacing and shape

- 4px base scale: 4, 8, 12, 16, 24, 32.
- Controls: 6px radius.
- Panels and dialogs: 10px radius.
- Status chips: compact 4px radius, not pills.
- Shadows reserved for dialogs and elevated menus; borders carry normal hierarchy.

### Surface hierarchy

- Canvas: graphite in dark mode, silver-white in light mode.
- Sidebar: one step darker or lighter than canvas.
- Work surface: neutral panel with hairline border.
- Raised surface: menus, dialogs, sticky action rail.
- Accent: restrained cobalt for focus and primary actions.
- Financial positive: emerald; warnings: amber; destructive: vermilion; unknown: violet-gray. Status always includes icon and text.

### Status grammar

- `ALLOTTED`: check icon, Allotted, positive semantic.
- `NOT_ALLOTTED`: minus-circle icon, Not allotted, neutral final semantic.
- `PENDING`: clock icon, Pending, informational semantic.
- `NOT_FOUND`: search-off icon, Not found, warning semantic.
- `UNKNOWN`: question icon, Unknown, indeterminate semantic.
- `VERIFICATION_REQUIRED`: shield-question icon, Verification required, attention semantic.
- `RATE_LIMITED`: timer icon, Rate limited, operational warning.
- `PROVIDER_UNAVAILABLE`: plug-off icon, Provider unavailable, operational danger.
- `RETRY_SCHEDULED`: rotate icon, Retry scheduled, informational semantic.
- `CANCELLED`: ban icon, Cancelled, muted final semantic.

### Interaction grammar

- Primary action records or begins a real operation.
- Secondary action previews, edits, reveals, or navigates.
- CHECK is outlined and labeled "Preview recommendation" in supporting text.
- SUBMIT is filled and labeled "Record investment" in supporting text.
- Historical application sits under Investments > Add as an explicit secondary mode.
- Human verification is presented as a normal continuation state, not an application failure.
- Every animated transition is disabled by reduced-motion preferences.

## Information architecture

Active navigation only:

1. Dashboard
2. Members
3. Investments
4. Check Allotment

Owner security state appears in the sidebar footer as non-navigation status. No News, Reports, Strategy Lab, Bot Training, or Settings destination is shown until the product implements it.

## Screens

- `mockups/dashboard.html` - compact sidebar, legitimate summary strip, quick actions, workflow distinction, activity and security hierarchy.
- Secure onboarding - implemented after the shared tracer is approved.
- Members - roster-first operations with progressive friend-account disclosure.
- Investments - current and historical modes with explicit preview vs record distinction.
- Check Allotment - candidate queue, provider state, account progress, report card, and manual provenance.
