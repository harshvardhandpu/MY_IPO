# Sanket IPO Implementation Plan

Authoritative requirements: `SANKET_IPO_MASTER_SOURCE.md`.

## Delivery rules

- Build vertical slices; every slice ends runnable and tested.
- Private accounting remains usable without network, GitHub, AI, news, or registrar availability.
- Write behavior tests before production logic.
- Update `docs/handoffs/CURRENT_STATE.md` after every meaningful checkpoint.
- Use synthetic sensitive data only.

## Phase 0 — Repository baseline

- [x] Verify repository, remote, branch, host tooling, GitHub authentication, and Tauri Linux libraries.
- [x] Install the v2 source at the canonical path.
- [x] Add ignore/secret-scan/CI safeguards.
- [x] Establish architecture, security, schema, sync, provider, profit, UI, and handoff documents.
- [x] Run baseline checks and commit.

## Phase 1 — Foundation

- [x] Tauri 2 / React / TypeScript / Vite shell with accessible desktop navigation.
- [x] Rust workspace with explicit domain, vault, sync, audit, and application boundaries.
- [x] Device identity and local settings.
- [x] SQLite initialization and migrations as a reconstructable projection.
- [x] Versioned append-only event envelope and audit service.
- [x] MemberVault and IntelligenceVault interfaces with disjoint capabilities.
- [x] Sync contracts/status model; no network dependency in local writes.
- [x] Runnable Linux smoke test and Windows-oriented CI checks.

## Phase 2 — First vertical slice

1. Encrypted member/friend identity with mandatory PAN validation and masked display.
2. Allocation-level accounting and deterministic profit/share calculations.
3. Invest → Check → Apply Recommendation → Submit.
4. Sanitized decision payload and placeholder algorithm clearly marked non-owner.
5. Provider contract, KFintech spike, durable allotment job, manual fallback, report card.
6. Encrypted proof pipeline and Git sync queue.
7. End-to-end and security acceptance tests.

## Later phases

3. Dashboards and analytics.  
4. Public news/intelligence.  
5. Provider-independent Report AI.  
6. Strategy Lab and reproducible backtesting.  
7. Signed Windows/Linux packaging, bootstrap, update, backup, and recovery.

## Quality gate per milestone

Implementation + tests + errors + docs + migrations + security review + manual smoke test + logical commit.
