# Current State

- **Branch:** `feature/foundation`
- **Latest implementation commit:** `798a420` (`feat(foundation): add runnable Tauri desktop core`)
- **Phase:** Phase 0 and Phase 1 complete; Phase 2A sensitive identity is next

## Completed

- Canonicalized the supplied v2 master source at `docs/planning/SANKET_IPO_MASTER_SOURCE.md`.
- Added repository ignore rules, local pre-commit secret scanning, Gitleaks configuration, Dependabot, and Linux/Windows CI.
- Added all required baseline architecture, data, event, sync, security, allotment, profit, UI, plan, and handoff documents.
- Added a Tauri 2 desktop shell with React, TypeScript 7, Vite, accessible navigation, dashboard hierarchy, and prominent Invest / Check Allotment actions.
- Added a Cargo workspace with domain, device settings, local index, member vault, intelligence vault, sync, audit, and Tauri adapter boundaries.
- Added stable device settings using atomic owner-only JSON persistence.
- Added embedded SQLite schema v1 with WAL, foreign keys, busy timeout, and owner-only Unix file permissions.
- Added schema-versioned, SHA-256-sealed event envelopes, stable role/sync-state serialization, and sensitive-access audit events that never accept the secret value.
- Added least-privilege Tauri capability configuration and a narrow `get_app_status` command.
- Completed a real Linux desktop smoke launch; the rendered shell, application data creation, migration version, and file permissions were verified.

## Tests and checks

Passed on the Linux development host:

- `npm run check` — secret scan, Biome formatting, TypeScript, 2 frontend tests, and 3 Python scanner tests.
- `npm run build` — Vite production bundle generated successfully.
- `cargo fmt --all -- --check` — run after formatting changes.
- `cargo clippy --workspace --all-targets -- -D warnings` — no warnings.
- `cargo test --workspace` — 9 Rust behavior/integration tests passed.
- Manual smoke — Tauri process launched, Vite served, native window rendered, SQLite migration returned version `1`, and settings/database files were mode `0600`.

## Security state

- No real PAN, UPI ID, proof, API key, or member data was used.
- The repository scanner and pre-commit hook are active through `core.hooksPath=.githooks`.
- Tauri exposes only `core:default`; there is no shell or arbitrary filesystem capability.
- MemberVault and IntelligenceVault are separate Rust capabilities with no AI dependency edge.
- SQLite and device settings are owner-only on Unix. Windows ACL hardening remains required before sensitive projections are introduced.

## Known limitations

- Dashboard values and charts are empty-state shell content; accounting persistence is not implemented yet.
- Invest and Check Allotment buttons are present but do not yet open workflows.
- Sync is currently a contract/status model only; no Git transport is running.
- CI is configured for Windows and Linux but has not run remotely until the branch is pushed.
- Sensitive identity encryption, OS credential storage, proof encryption, registrar workers, and AI DTOs are intentionally deferred to their planned phases.

## Important architecture decisions

- Tauri modular monolith; no network microservices.
- Local event persistence is the write success boundary; SQLite and Git sync are derived/asynchronous.
- Money will use integer paise; percentages will use integer basis points.
- Private, sensitive-automation, and public/AI domains remain structurally disjoint.
- Durable event payloads are allowlisted Rust enums rather than arbitrary JSON at the command boundary.

## Next exact tasks — Phase 2A

1. Add the crypto crate and key-provider abstraction without committing keys.
2. Add mandatory PAN validation, masking, redacted secret handling, and generated synthetic fixtures.
3. Add encrypted member/friend identity envelopes and purpose-scoped `with_pan` access.
4. Add MemberVault encrypted-record persistence and audit-on-access integration.
5. Add adversarial tests proving PAN/UPI/proofs/private member objects cannot enter AI request DTOs or logs.

## Commands

```bash
npm install
npm run check
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run tauri:dev
```
