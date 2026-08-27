# UI Screen Map

## App shell

Desktop sidebar plus compact three-dot menu, top sync/offline indicator, role-aware actions, accessible keyboard navigation, and actionable error toasts. Owner-only navigation is hidden and command authorization is still enforced in Rust.

## Primary screens

- **Unlock/onboarding:** GitHub membership state, device registration, local PIN, secure identity setup.
- **Dashboard:** group metrics, charts, current applications, top IPOs, members, activity; prominent `Invest` and `Check Allotment`.
- **Invest:** daily capital; repeatable manual IPO rows; own account selected by default; friend toggles; arithmetic warnings; separate `CHECK` and `SUBMIT`.
- **Recommendation:** ranked IPOs, criterion results, account-count/ratio suggestion, confidence, public provenance, apply action, explicit placeholder label until owner algorithm exists.
- **Allotment queue/report:** eligible IPOs, durable job progress, per-account masked rows, human-verification/manual/retry actions, evidence, explicit profit basis.
- **Members:** core member list, primary account, friend add/archive, consent/share settings, audit history.
- **Investments / IPO detail:** allocation-level truth, proofs, allotments, profit/share calculation, filters and charts.
- **News:** India-first public intelligence cards with provenance and optional linked media.
- **Report:** Algorithm Mode and separate experimental Suggest Mode.
- **Strategy Lab:** public-data-only experiments, ₹60k simulation, validation metrics and failed strategies.
- **Settings/Admin:** devices, members, vault/sync health, provider/algorithm settings, backup/recovery.

## State design

Every data screen defines loading, empty, offline/stale, partial, error, and success states. Local data remains visible when remote services fail.
