# Architecture: Aether UI Redesign

## Fit

The redesign stays inside the existing React frontend and its stylesheet.

- `apps/desktop/src/App.tsx` keeps the current view state, command bridge, form state, validation, and Tauri command names. Small presentation components may be extracted only when they remove repeated status, icon, or field markup.
- `apps/desktop/src/styles.css` becomes the single semantic token and component-style source. Existing class names are retained where practical to reduce JSX churn.
- Existing frontend tests remain the behavioral contract. New assertions cover navigation semantics, status distinctions, secure masking, and historical-entry labeling.
- Design docs and throwaway mockups stay under `docs/plans/aether-ui-redesign/`.

Frozen files include every path under `crates/`, `apps/desktop/src-tauri/`, migrations, schemas, provider adapters, identity services, security policy, and authorization gates.

## Endpoints

None. No Tauri command is added, removed, renamed, or widened.

The Dashboard may reuse the existing safe `list_allotment_candidates` command for current-activity rows. This is a frontend read of an already approved DTO, loaded independently so a candidate-read failure cannot block application startup.

## Data

No schema or projection changes.

Existing safe frontend values are sufficient:

- Dashboard totals and counts from `get_dashboard`.
- Masked member and friend identity from `list_members` and `list_friends`.
- Current and historical investment inputs through existing command request shapes.
- Application, registrar, provider-health, progress, last-checked, and job state from `list_allotment_candidates`.
- Account status, masked PAN, source, provenance, allotment quantity, amount, timestamps, and profit basis from `get_allotment_report`.
- Key provider and release-blocker status from `get_security_status`.

The UI never receives or caches plaintext PAN after onboarding. Existing secure input submission remains unchanged.

## Flow

### Application shell

1. `App` loads current members, friends, and dashboard data as it does today.
2. `AppShell` renders only Dashboard, Members, Investments, and Check Allotment.
3. The sidebar footer shows non-interactive local-vault/security language, never invented provider health.
4. View switching remains local React state with no routing or navigation package.

### Dashboard activity

1. Dashboard paints from existing projection-backed totals immediately.
2. A non-blocking frontend read requests `list_allotment_candidates`.
3. Available rows render in a compact activity table; unavailable or empty data renders an honest empty state.
4. Quick actions switch to existing Investments or Check Allotment views.

### Investments

1. Current investment remains the default mode.
2. CHECK continues to call `check_recommendation` with the existing safe request.
3. Recommendation output remains visually labeled as preview/development output.
4. SUBMIT continues to call `submit_investment` only after a recommendation exists.
5. Historical entry remains an explicit secondary mode and continues to call `record_historical_application` unchanged.

### Allotment

1. Candidate selection, provider-independent job start, polling, cancellation, human verification, manual provenance, and profit-basis flows remain unchanged.
2. Presentation maps every state to one shared status grammar.
3. UNKNOWN, NOT_FOUND, and NOT_ALLOTTED receive distinct text, icon, and semantic token combinations.
4. Automated and manual provenance are rendered as explicit source labels.

## Styling strategy

- Native CSS variables, Grid, sticky positioning, media queries, and semantic HTML.
- No design-system package, icon dependency, animation library, chart library, router, or state manager.
- One 4px spacing scale, 6px controls, 10px panels/dialogs, border-led hierarchy, and tabular numbers.
- System color preference maps the same semantic tokens to light and dark palettes.
- Motion is limited to 120-180ms control feedback and disclosure transitions, with reduced-motion overrides.
- Responsive target is desktop-first: dense at 1280px+, adaptive at 960-1279px, and safe single-column fallback below 960px without turning the app into a mobile marketing stack.

## External

None. No web fonts, remote assets, analytics, CDNs, or new npm dependencies.

## Preservation checks

- `git diff --name-only 70a57bc..HEAD` must contain frontend UI/tests and design docs only.
- Existing frontend flow tests must pass without changing command names or secure request fields.
- Repository secret scan and desktop build must pass.
- Review must explicitly confirm no Rust/domain/schema/security/provider diff and no full PAN rendering.
