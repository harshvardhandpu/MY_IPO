# Vertical Slices: Aether UI Redesign

1. **Dashboard tracer** - introduce the semantic token system, compact app shell, four-item accessible navigation, summary strip, safe activity read, decision boundary, and verified light/dark responsive dashboard.
2. **Secure people surfaces** - apply the approved grammar to first-run onboarding and Members while preserving secure fields, masked output, consent, friend eligibility, archive behavior, and progressive disclosure.
3. **Investment workspace** - redesign current and historical modes, IPO/account rows, recommendation preview, and sticky CHECK/EDIT/SUBMIT rail without changing command payloads or validation.
4. **Allotment operations console** - add the shared status grammar, candidate queue, provider health, progress, report rows, automated/manual provenance, verification continuation, and profit-basis disclosure.
5. **System QA and finish** - test light/dark parity, narrow desktop fallback, keyboard order, focus, reduced motion, empty/error/loading states, visible text, frontend gates, secret scan, and frozen-backend diff; then obtain independent review and commit.

## Slice boundary rules

Each slice must:

- Modify frontend presentation/tests and design status only.
- End with `typecheck`, focused frontend tests, and production build green.
- Preserve exact Tauri command names and mutation request bodies.
- Leave full PAN absent from rendered post-save state.
- Avoid new dependencies, fabricated features, fabricated financial values, and decorative charts.
- Stop for trajectory approval before the next slice.
