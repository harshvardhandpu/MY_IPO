# SANKET-R1 — Security Closure Evidence

- **Recorded:** 2026-09-07T13:57:00+05:30
- **Card:** `t_952b4fdc`
- **Scope:** security closure and authentication implementation acceptance; no feature expansion, provider integration, financial mutation, commit, or push.

## Authority state

- `SEC-AUTH-01`: native `done`, design-only authority result `APPROVE_MINIMAL_DESIGN`; not reopened.
- `BUG-AUTH-01`: native `done` after direct `ASTRA_HIGH` approval.
- BUG approval artifact: `reports/release-readiness/bug-auth-01-astra-high-approved.json`
- BUG approval SHA-256: `bd31435ac025a1d820b4e9620f9b8b21345c117557b33918ec12d1b2b1e72896`
- BUG evidence SHA-256: `883423408e10a7b0dadfa557be707802679b6c5b79addb3dcdc55d984f7f599b`
- Authority route: `ASTRA_HIGH`, `gpt-6-astra`, `openai-codex`, session `20260907_135117_129165`; result `APPROVED` for BUG implementation acceptance only.

## Verified implementation/security evidence

- Event/vault reconstruction is authoritative; SQLite remains a projection/query surface.
- Argon2id credential hashing and verification use zeroized transient inputs.
- Signup is private/invite-controlled; invite persistence is digest-only, expiring, and one-time.
- Approval/revocation are event-derived; login is generic for unknown, wrong, pending, and revoked accounts.
- Failed-login backoff is bounded; session validation is server-side and rejects expiry/revocation.
- Native session ownership is in Tauri state; native token installation occurs only after accepted login; logout removes native session before the logout event.
- Protected member/investment/allotment/security/lookup/catalog commands require authenticated actor validation and fail closed when auth state is unavailable.
- Onboarding binds the member identity to the authenticated account rather than a UI-generated identifier.
- Password/invite/session DTO debug/wire boundaries are redacted; serialized login response omits the native session token; auth events contain metadata only.

## Focused execution gates

| Gate | Result |
|---|---|
| `cargo test -p sanket-desktop --lib auth::tests` | PASS, 8 tests |
| `cargo test -p sanket-desktop --lib native_auth_command_tests` | PASS, 5 tests |
| `cargo test -p sanket-identity-security --test password_credentials` | PASS, 5 tests |
| `cargo test -p sanket-domain --test auth_events` | PASS, 3 tests |
| `cargo test -p sanket-member-vault --test auth_initialization` | PASS, 2 tests |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `npm run check` | PASS: secrets, format, typecheck, 10 frontend tests, 4 Python tests |
| `npm run secrets` | PASS: 211 files checked |
| `git diff --check` | PASS |

## Explicit boundary evidence

- No PAN, credential value, keyring value, live provider/MUFG request, financial mutation, commit, or push occurred.
- Tests use ephemeral synthetic material only; no sensitive fixture was retained.
- The current worktree is intentionally dirty and remains on protected HEAD `d3864557ee56282214733d44149cb2ed1fbc0ee6`; repository changes predate this evidence-only update.

## Remaining release separation

This card closes the security/auth implementation acceptance boundary only. Linux package installation/keyring/clean-profile smoke, Windows build/Credential Manager/clean-profile smoke, restart/persistence release-package evidence, cross-platform acceptance, and final release authority remain separate downstream gates. No platform or final release approval is implied.
