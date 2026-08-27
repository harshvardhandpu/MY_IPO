# Current State

- **Branch:** `feature/sensitive-identity`
- **Base:** `83811b3` (`docs(handoff): record completed foundation milestone`) on `feature/foundation`
- **Phase:** Phase 2A (Sensitive Identity Security) complete; Phase 2B (CoreMember/FriendAccount onboarding) is next

## Completed — Phase 2A

### New crate: `sanket-identity-security`
- `Pan` domain type: format validation (5 letters + 4 digits + 1 letter), case/whitespace normalization, `ABCDE****F` masking. No `Serialize` impl (compile-time leak prevention). `Display`/`Debug` render only masked/redacted forms.
- `MaskedPan`: safe serializable display type.
- `Redacted<T>`: generic zeroize-on-drop wrapper; `Display`/`Debug` always render `[REDACTED]`.
- `IdentityKey`: 32-byte key, zeroized on drop, explicit construction.
- `IdentityCipher`: XChaCha20-Poly1305 AEAD (chacha20poly1305 crate), random 24-byte nonce per encryption.
- `EncryptedIdentityEnvelope`: versioned (v1), algorithm-tagged, key-id-referenced, serde-serializable ciphertext-only struct.
- `KeyProvider` trait + `InMemoryKeyProvider` (dev/test). OS keyring providers abstracted, deferred to packaging.
- `SensitiveIdentityRecord`: holds only masked PAN + encrypted envelope; no plaintext field.
- `SensitivePurpose` enum: `AllotmentCheck` authorized; `Unknown` rejected before decryption.
- `SensitiveIdentityService::with_pan(record, purpose, actor, closure)`: transient decrypt → closure → drop → audit. No long-lived PAN escape.
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
- New test proves the allowlist boundary.

### Security invariant regression test
- `crates/member-vault/tests/security_invariant.rs`: full lifecycle — encrypt → persist → purpose-scoped access → audit → walk all persisted files → assert no plaintext PAN anywhere. Permanent regression.

## Tests and checks (all passing)

- `cargo test --workspace`: **50 Rust tests** (up from 9 at Phase 1).
  - identity-security: 27 (4 unit + 19 security + 4 logging)
  - member-vault: 8 (7 persistence + 1 security invariant)
  - intelligence-vault: 6 (AI boundary)
  - foundation crates: 9 (unchanged)
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `npm run check`: secret scan (77 files), Biome format, TypeScript, 2 frontend tests, 4 Python scanner tests — all pass.
- `npm run build`: Vite production bundle OK.

## Security invariants proven by tests

1. Encrypted identity round-trip works; wrong key/corrupted ciphertext/tampered nonce all fail authentication.
2. PAN masking: `ABCDE****F`; no interior digits; Debug/Display redacted.
3. Purpose-scoped access: `AllotmentCheck` works; `Unknown` rejected before decryption.
4. Audit contains account_id + purpose + time; never PAN.
5. AI payload cannot carry PAN/UPI (compile-time + runtime fail-closed).
6. Validation errors never echo input.
7. Persisted files contain no plaintext PAN (byte-level walk).
8. Secret scanner catches PAN leakage in non-test paths.

## Deferred (documented, not mocked)

| Item | Status |
|---|---|
| Windows Credential Manager KeyProvider | ABSTRACTED (trait ready); DEFERRED to packaging |
| Linux Secret Service KeyProvider | ABSTRACTED (trait ready); DEFERRED to packaging |
| Windows ACL hardening for vault dir | DEFERRED to packaging |
| Key rotation tooling | Envelope design supports it; no rotation command yet |

## Known limitations

- Dashboard UI unchanged; no member/friend creation UI yet (Phase 2B).
- `SensitiveIdentityService` audit is in-memory until wired to event persistence (Phase 2B wiring).
- No OS keyring integration yet; dev/test uses `InMemoryKeyProvider` only.

## Next exact tasks — Phase 2B

1. `CoreMember` domain type: id, display name, role, primary account id, sensitive record ref, masked PAN, consent metadata, status.
2. `FriendAccount` domain type: id, owner member id, label, sensitive record ref, masked PAN, broker, share eligibility/basis points, status.
3. Member onboarding flow: PAN mandatory, validation, encryption, persistence, masked display.
4. Friend account creation: PAN mandatory for investable friends, 10% default share eligibility.
5. Primary account designation.
6. UPI fields (encrypted or masked; not in AI payloads).
7. Archive-not-delete for friends.
8. Member-wide notification event on friend add/archive.
9. Deterministic accounting objects (paise/basis points).
10. Wire `SensitiveAccessAudit` to MemberVault event persistence.

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
