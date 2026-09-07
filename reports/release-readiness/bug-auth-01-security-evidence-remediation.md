# BUG-AUTH-01 — ASTRA_HIGH Evidence Remediation

- **Recorded:** 2026-09-07T13:48:37+05:30
- **Purpose:** Address the ASTRA_HIGH `REVISE` evidence request without changing production behavior or widening feature scope.
- **Authority request:** `20260907_134526_6e85f6`
- **Authority route:** `openai-codex / gpt-6-astra` (explicit ASTRA_HIGH route)
- **Health probe:** `20260907_134358_98e3d9` returned the exact requested JSON; exit 0.
- **Protected HEAD:** `d3864557ee56282214733d44149cb2ed1fbc0ee6`
- **Worktree:** intentionally dirty pre-existing state; no reset, commit, push, or unrelated edit.
- **git diff --check:** PASS.

## Source-level authority and enforcement findings

### Event/vault authority

- `apps/desktop/src-tauri/src/auth.rs:378-385` reconstructs authentication state from `MemberVault::list_events()` and sorts the verified event stream before deriving state.
- `auth.rs:400-420` bootstraps the owner by hashing the password with the pinned credential primitive and appending `EventPayload::OwnerBootstrapped`; no SQLite projection is consulted.
- `auth.rs:422-461` permits invite issuance only for an active manager, persists only `invite_digest` and expiry in `InviteIssued`, and returns the one-time secret only to the caller.
- `auth.rs:463-495` accepts signup only for an existing, unused, unexpired invite with a matching digest and account identity, then appends `SignupPending` with the password verifier.
- `auth.rs:497-551` derives approval/revocation from reconstructed account state and appends `AccountApproved`/`AccountRevoked` events.
- `apps/desktop/src-tauri/src/lib.rs:75-92` creates `AuthService` from the vault path and holds the active session only in native `AppState`; the compatibility constructor has no auth vault/session.

### Credential and session boundary

- `auth.rs:553-629` applies generic account lookup, dummy verification for unknown logins, pinned verifier/version checks, active-account checks, bounded backoff, and native session creation. A failed event append removes the provisional session and returns the generic rejected result.
- `auth.rs:631-644` validates the native session against expiry and reconstructed active account status; expiry or revocation removes the live session and returns `None`.
- `auth.rs:646-663` removes the session before appending `SessionLoggedOut`, so subsequent validation rejects the former token.
- `lib.rs:138-169` owns the active token in a native mutex, zeroizes replaced/copied values, and clears invalid sessions on failed validation.
- `lib.rs:228-309` installs a native session only after an accepted login/bootstrap response and clears it on logout.

### Complete protected Tauri-command inventory

`lib.rs:720-756` registers the complete command surface. Authentication and protection are applied as follows:

- **Auth lifecycle:** `bootstrap_owner`, `login`, `logout`, `issue_invite`, `complete_signup`, `approve_account`, `revoke_account`, `get_auth_status`.
- **Protected member/investment reads and writes:** `onboard_member`, `add_friend`, `archive_friend`, `list_members`, `list_friends`, `check_recommendation`, `submit_investment`, `record_historical_application`, `void_submitted_session`, `get_dashboard`.
- **Protected allotment operations:** `list_allotment_candidates`, `start_allotment_check`, `get_allotment_report`, `cancel_allotment_job`, `record_manual_allotment`, `estimate_profit`.
- **Protected security/lookup operations:** `get_security_status`, `get_lookup_authorization_status`, `authorize_real_investor_lookup`.
- **Protected provider-credential/catalog operations:** `get_upstox_connection_status`, `connect_upstox_analytics_token`, `replace_upstox_analytics_token`, `disconnect_upstox`, `list_available_ipos`, `refresh_ipo_catalog`, `get_ipo_details`.
- **Unauthenticated status:** `get_app_status` only.

Enforcement trace:

- `lib.rs:157-176`: `authenticated_actor` requires a live native token and `validate_session`; `manager_actor` additionally requires a manager role.
- `lib.rs:384-429`: member onboarding, friend creation, and archiving bind/compare the request actor to the authenticated native actor.
- `lib.rs:432-605`: all member, investment, dashboard, allotment, security, and lookup paths call `authenticated_actor` before application access.
- `lib.rs:607-620`: real-investor authorization additionally requires `Role::Owner` and overwrites the request actor.
- `lib.rs:623-663`: provider credential access requires authenticated/manager state and returns a non-connected DTO on denial; supplied tokens are taken and cleared before storage.
- `lib.rs:665-690`: IPO catalog reads require an authenticated actor before provider-service access.

## Named focused assertions and fresh results

- `cargo test -p sanket-desktop --lib auth::tests`: **PASS**, 8 tests.
  - `owner_bootstrap_reconstructs_from_vault_and_never_uses_projection_counts`
  - `invite_is_private_one_time_expiring_and_persists_only_a_digest`
  - `expired_invite_is_rejected_without_persisting_the_secret`
  - `approval_and_revocation_reconstruct_account_state_from_events`
  - `login_is_generic_for_unknown_wrong_pending_and_revoked_accounts`
  - `failed_login_backoff_is_bounded_and_session_validation_is_server_side`
  - `auth_dtos_do_not_debug_or_serialize_transient_secrets`
  - `auth_events_are_metadata_only`
- `cargo test -p sanket-desktop --lib native_auth_command_tests`: **PASS**, 5 tests.
  - `compatibility_constructor_has_no_auth_vault_or_session`
  - `rejected_login_cannot_install_native_session`
  - `accepted_login_is_visible_only_through_native_auth_status`
  - `onboarding_binds_member_identity_to_authenticated_account`
  - `unavailable_auth_state_fails_closed_for_actor_validation`
- `cargo test -p sanket-identity-security --test password_credentials`: **PASS**, 5 tests.
- `cargo test -p sanket-domain --test auth_events`: **PASS**, 3 tests.
- `cargo test -p sanket-member-vault --test auth_initialization`: **PASS**, 2 tests.
- `cargo test --workspace`: **PASS**.
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `cargo fmt --all -- --check`: **PASS**.
- `npm run check`: **PASS** (secret scan, format, typecheck, 10 frontend tests, 4 Python tests).
- `npm run secrets`: **PASS** (211 files).

## Secret and sensitive-operation boundary

- `auth.rs:43-166` explicitly redacts password, invite secret, and session token fields from `Debug`; `LoginResponse.session_token` is `serde(skip_serializing)`.
- `auth.rs:973-1014` asserts transient secrets are absent from debug output and the serialized login response.
- `auth.rs:1016-1050` asserts auth events contain metadata/digests only and exclude plaintext passwords, invite/session secrets, and PAN-shaped data.
- No PAN, credential value, keyring value, live provider/MUFG request, financial mutation, commit, or push occurred.

## Current artifact hashes

- `apps/desktop/src-tauri/src/auth.rs` — `17af8871620b286a450da2984a001989441e44c47ecf5494a6cebfcf6bd686e4`
- `apps/desktop/src-tauri/src/lib.rs` — `9e46638f67ad44942c3f9bbafe917ecf0245ca0d7f249e6bd21a1ca68a92314d`
- `crates/identity-security/src/password.rs` — `c456d1f4a0f3183fb07a88ee5106f845764ff0828564dcf1449ec388c74c7891`
- `crates/domain/src/lib.rs` — `1fd8c777775313fe74cca855a6f975ebbc738885e6afab3bfbeac6797e86407a`
- `crates/member-vault/src/lib.rs` — `9e70537f3f7db9572b3cefb6b5f5b362d9212caa31c6ebb2c5e8a1d6e8041e3a`
- `apps/desktop/src/App.tsx` — `eac4202e12af726514a8661435491c17d45c3deaad9dcd8effae26116835c298`
- `reports/release-readiness/astra-governance-policy.md` — `83e4269506b9398339fe355b2a49fedfc20a138d24179e30fe6cf6536f956245`
- `reports/release-readiness-checkpoint.md` at capture — `580684be9c8dfa0379b033e831903a5ae5cc9ef449cb2338ed7fe9572e12823a`

## Remediation status

This is evidence-only remediation for the ASTRA_HIGH `REVISE`; no production code changed during this remediation. The board has only the `default` profile and no independently verified free worker route was available, so no worker claim is asserted. BUG-AUTH-01 remains on the controlled review path and must be resubmitted to ASTRA_HIGH with this report and the fresh focused-test results. Platform packaging/clean-profile evidence and final release authority remain separate gates.
