# Sensitive Identity

Phase 2A implementation status: **IMPLEMENTED and TESTED** on Linux. OS credential-store integration is **ABSTRACTED** (trait in place) with native providers **DEFERRED** to packaging.

## PAN lifecycle

1. **Entry** — user types PAN during onboarding. `Pan::parse` validates format (5 letters, 4 digits, 1 letter), normalizes case/whitespace. Invalid input produces a static error that never echoes the input.
2. **Masking** — `Pan::mask()` → `MaskedPan` (`ABCDE****F`). `Display` and `Debug` on `Pan` render only the masked form. `Pan` does not implement `Serialize`; serializing it is a compile error.
3. **Encryption** — `IdentityCipher::encrypt(pan_bytes, key_id)` → `EncryptedIdentityEnvelope` (XChaCha20-Poly1305, random 24-byte nonce, version 1). The plaintext is consumed and never stored in the record struct.
4. **Persistence** — `MemberVault::store_member_identity` / `store_friend_identity` writes the envelope JSON atomically (temp file + fsync + rename), owner-only (`0600` on Unix), under `_secure_identity/<id>.enc` or `_secure_identity/friends/<id>.enc`.
5. **Access** — `SensitiveIdentityService::with_pan(record, purpose, actor, closure)` decrypts transiently, runs the closure, drops the plaintext, and records an audit event. No long-lived PAN object escapes.
6. **Audit** — `SensitiveAccessAudit { account_id, purpose, occurred_at }`. Never contains the PAN value.

## Where plaintext PAN may exist

- Inside the `with_pan` closure, for the duration of the callback only.
- In user input fields before encryption (transient UI state; never persisted).

## Where plaintext PAN must never exist

- Markdown frontmatter, event JSON payloads, SQLite projections, Git commit messages, filenames, crash reports, telemetry, AI prompts, log output, error messages, Debug/Display formatting.

## Encryption envelope

```rust
EncryptedIdentityEnvelope {
    version: u16,        // currently 1; unknown versions rejected on load
    algorithm: String,   // "XChaCha20-Poly1305"
    key_id: String,      // stable identifier resolved via KeyProvider
    nonce: Vec<u8>,      // 24 random bytes per encryption
    ciphertext: Vec<u8>, // AEAD ciphertext + Poly1305 tag
    created_at: String,  // RFC 3339 UTC
}
```

Key rotation: a new envelope is written with a new `key_id`; old envelopes remain decryptable as long as the old key is available in the provider.

## Key-provider model

```rust
trait KeyProvider: Send + Sync {
    fn key(&self, key_id: &str) -> Option<IdentityKey>;
}
```

| Status | Provider | Notes |
|---|---|---|
| IMPLEMENTED + TESTED | `InMemoryKeyProvider` | Development/test only. Keys never touch disk. |
| ABSTRACTED | `KeyProvider` trait | Production implementations plug in here. |
| DEFERRED | Windows Credential Manager | Packaging phase; `keyring` crate candidate. |
| DEFERRED | Linux Secret Service | Packaging phase; `keyring` crate candidate. |

Production code must never depend directly on `InMemoryKeyProvider`.

## Purpose-scoped access

```rust
enum SensitivePurpose { AllotmentCheck, #[serde(other)] Unknown }
```

- `AllotmentCheck` is the only authorized purpose in Phase 2A.
- `Unknown` (any unrecognized value) is rejected before decryption.
- Future purposes require an explicit enum variant and a security review.

## Audit model

Every `with_pan` call produces a `SensitiveAccessAudit`. The audit is appended to the event log via `sanket-audit::sensitive_identity_accessed` (account_id + purpose + actor/device + time). The PAN value is structurally absent from the audit type.

## Remaining OS-specific work (DEFERRED)

- Windows: Credential Manager-backed `KeyProvider` via `keyring` crate. ACL hardening for vault directory.
- Linux: Secret Service-backed `KeyProvider` via `keyring` crate.
- Both: key generation on first run, secure key deletion on member removal.
