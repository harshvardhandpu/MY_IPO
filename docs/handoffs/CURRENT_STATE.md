# Current State

- **Branch:** `feature/sensitive-identity`
- **Base:** `83811b3` (`docs(handoff): record completed foundation milestone`) on `feature/foundation`
- **Phase:** Phase 2A complete; Phase 2B (CoreMember/FriendAccount onboarding domain) complete; Phase 3 (investment sessions & IPO application) is next

## Completed — Phase 2A (Sensitive Identity Security)

### New crate: `sanket-identity-security`
- `Pan` domain type: format validation (5 letters + 4 digits + 1 letter), case/whitespace normalization, `ABCDE****F` masking. No `Serialize` impl (compile-time leak prevention). `Display`/`Debug` render only masked/redacted forms.
- `MaskedPan`: safe serializable display type, plus `from_parts` for profile-only construction.
- `Redacted<T>`: generic zeroize-on-drop wrapper; `Display`/`Debug` always render `[REDACTED]`.
- `IdentityKey`: 32-byte key, zeroized on drop, explicit construction.
- `IdentityCipher`: XChaCha20-Poly1305 AEAD (chacha20poly1305 crate), random 24-byte nonce per encryption.
- `EncryptedIdentityEnvelope`: versioned (v1), algorithm-tagged, key-id-referenced, serde-serializable ciphertext-only struct.
- `KeyProvider` trait + `InMemoryKeyProvider` (dev/test). OS keyring providers abstracted, deferred to packaging.
- `SensitiveIdentityRecord`: holds only masked PAN + encrypted envelope; no plaintext field.
- `SensitivePurpose` enum: `AllotmentCheck` authorized; `Unknown` rejected before decryption.
- `SensitiveIdentityService::with_pan` / `with_identity`: transient decrypt → closure → drop → audit. No long-lived PAN escape.
- `SensitiveAccessAudit`: account_id + purpose + timestamp only; structurally cannot contain PAN.

### MemberVault encrypted persistence
- `MemberVault::store_member_identity` / `store_friend_identity`: atomic writes (temp + fsync + rename), owner-only `0600` on Unix.
- Deterministic layout: `_secure_identity/<member-id>.enc`, `_secure_identity/friends/<friend-id>.enc`.
- Path-traversal rejection on ids. Envelope version validation on load. Malformed JSON fails safely.
- `append_event` for audit event persistence.

### AI boundary hardening
- `InvestmentDecisionPayload` in `sanket-intelligence-vault`: sanitized scalar fields only, fail-closed `assert_safe()` rejecting PAN-like and UPI-like tokens.
- Adversarial tests prove PAN/UPI tokens rejected; private field names structurally absent.

### Logging security
- Validation errors are static descriptions; never echo input.
- `Redacted` renders identically regardless of wrapped value.
- Masked PAN contains zero interior digits.

### Secret scanner expansion
- Test-path allowlist for synthetic PAN fixtures (narrow: only `tests/` paths).
- Non-test paths still fail closed on any format-valid PAN.

### Security invariant regression test
- `crates/member-vault/tests/security_invariant.rs`: full lifecycle — encrypt → persist → purpose-scoped access → audit → walk all persisted files → assert no plaintext PAN anywhere.

## Completed — Phase 2B (Member/Friend Onboarding Domain)

### IdentitySecret (PAN + UPI payload)
- `IdentitySecret { pan, upi_id }`: unified encrypted payload, JSON wire form inside the envelope.
- `SensitiveIdentityRecord::encrypt_identity` + `SensitiveIdentityService::with_identity`.
- Backward compatible: `with_pan` decodes both raw-PAN (Phase 2A) and JSON (Phase 2B) envelopes.
- Fail-closed UPI validation (non-empty, no whitespace, exactly one `@`).
- `IdentitySecret` Debug redacts UPI; envelope serialization hides both PAN and UPI.

### Deterministic accounting types (domain)
- `Money`: integer paise, `from_paise`/`from_rupees`, checked add/sub (fails closed on negative/overflow), `₹X.YY` display.
- `BasisPoints`: integer bp (10_000 = 100%), range-validated, `share_of` with deterministic truncation (share + remainder == amount, proven by test).
- `BasisPoints::friend_share_default()` = 1_000 (10%).

### CoreMember / FriendAccount (domain)
- `CoreMember::onboard`: PAN mandatory (masked form required by value), role, sensitive record ref, status Active.
- `designate_primary_account`: exactly one primary, redesignation replaces.
- `FriendAccount::create`: PAN mandatory, owner ref, default 10% share, `set_share_basis_points` range-enforced.
- `MemberStatus { Active, Archived }` — archive-not-delete: no Deleted variant, no delete methods, archived friends not investable.
- `EventPayload::FriendAccountAdded` / `FriendAccountArchived`: member-wide notification events carrying ids/label/share only, never PAN.

### MemberVault profile persistence
- `store/load_member_profile`, `store/load_friend_profile`: atomic owner-only JSON under `_profiles/members/` and `_profiles/friends/`.
- Profiles contain only masked PAN (test asserts no interior digits).
- `list_member_ids` / `list_friend_ids`.
- Path-traversal rejection on profile ids.

### Audit wiring (item 10)
- `sanket_audit::access_audit_event`: converts `SensitiveAccessAudit` → sealable `EventEnvelope`.
- End-to-end test: access → audit → seal → vault append → persisted event has purpose, never PAN.

## Tests and checks (all passing)

- `cargo test --workspace`: **83 Rust tests**.
  - identity-security: 32 (4 unit + 19 security + 4 logging + 5 identity-secret)
  - domain: 24 (2 event + 2 foundation + 11 money + 9 members)
  - member-vault: 15 (7 persistence + 7 profiles + 1 security invariant)
  - intelligence-vault: 6 (AI boundary)
  - audit: 2 (sensitive access + wiring)
  - other foundation crates: 4
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `npm run check`: secret scan (80 files), Biome format, TypeScript, 2 frontend tests, 4 Python scanner tests — all pass.
- `npm run build`: Vite production bundle OK.
- Linux desktop smoke: app launches, WebView renders, settings/SQLite owner-only.

## Security invariants proven by tests

1. Encrypted identity round-trip works; wrong key/corrupted ciphertext/tampered nonce all fail authentication.
2. PAN masking: `ABCDE****F`; no interior digits; Debug/Display redacted.
3. Purpose-scoped access: `AllotmentCheck` works; `Unknown` rejected before decryption.
4. Audit contains account_id + purpose + time; never PAN.
5. AI payload cannot carry PAN/UPI (compile-time + runtime fail-closed).
6. Validation errors never echo input.
7. Persisted files contain no plaintext PAN (byte-level walk).
8. Secret scanner catches PAN leakage in non-test paths.
9. Profile files contain only masked PAN.
10. Notification events carry no PAN.
11. Money splits are exact: share + remainder == amount for all tested amounts.

## Deferred (documented, not mocked)

| Item | Status |
|---|---|
| Windows Credential Manager KeyProvider | ABSTRACTED (trait ready); DEFERRED to packaging |
| Linux Secret Service KeyProvider | ABSTRACTED (trait ready); DEFERRED to packaging |
| Windows ACL hardening for vault dir | DEFERRED to packaging |
| Key rotation tooling | Envelope design supports it; no rotation command yet |
| Member/friend creation UI | Domain + persistence ready; Tauri commands/UI next phase |

## Known limitations

- Dashboard UI unchanged; onboarding flows exist as domain + vault APIs, not yet as Tauri commands/UI.
- No OS keyring integration yet; dev/test uses `InMemoryKeyProvider` only.
- UPI validation is minimal (format sanity), not full NPCI validation.

## Next exact tasks — Phase 3 (Investment Sessions & IPO Application)

1. `InvestmentSession` domain type: id, actor/member, declared capital paise, status, timestamps.
2. `IPO` domain type: id, typed/canonical name, issue metadata, registrar mapping, public provenance.
3. `IPOApplication`: id, session id, IPO id, member id, status, expected allotment metadata.
4. `InvestmentAllocation`: application/account linkage, amount paise, lots/shares, friend-share eligibility.
5. Session lifecycle events (opened, capital declared, closed) in `EventPayload`.
6. MemberVault persistence for sessions/applications (safe metadata only).
7. Deterministic lot-size calculation from price band + capital.
8. Tauri commands exposing member/friend onboarding + session creation to the UI.

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
