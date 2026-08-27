# Threat Model

## Assets

Full PAN, UPI IDs, member/friend identity mapping, allocations, proofs, profits, API credentials, device keys, audit history, and repository access.

## Trust zones

1. **Private member zone:** MemberVault, local projection, proofs, accounting services.
2. **Sensitive automation zone:** local allotment process with one-account/one-purpose decryption.
3. **Public intelligence zone:** public sources, IntelligenceVault, AI gateway, Strategy Lab.
4. **External zone:** GitHub, AI providers, registrar sites, news sources.

## Primary threats and controls

| Threat | Control |
|---|---|
| Plaintext PAN in Git/disk | encrypted identity envelopes, allowlisted schemas, repository scanner |
| Overbroad local cache access | app-private location and explicit owner-only file permissions on Unix; Windows ACL hardening before sensitive projections |
| PAN/UPI/proofs sent to AI | disjoint request DTOs, no MemberVault dependency in AI crates, adversarial tests |
| Sensitive values in logs/crashes | structured redaction, no debug serialization of secrets, scanner tests |
| Proof disclosure | compress → hash → encrypt; lazy authorized decrypt; no AI/media indexing |
| Malicious registrar page | domain allowlist, isolated replaceable worker, minimal lookup data, session destruction |
| CAPTCHA/OTP bypass | explicit `NEEDS_HUMAN_VERIFICATION`; no bypass logic |
| Wrong provider parsing | known-pattern normalization; unknown maps to `UNKNOWN`; fixture contract tests |
| Git conflict/data loss | append-only per-device events, stable IDs, hashes, quarantine invalid events |
| Compromised former member | revoke Git/device access and rotate keys; document that old local copies cannot be erased remotely |
| Secret leakage | OS credential store, encrypted enrollment bundles, no plaintext `.env` in Git |
| Path traversal | canonicalize and enforce configured vault roots in Rust |
| Dependency compromise | lockfiles, CI audit, minimal dependencies, signed releases |

## Security invariants

- External AI cannot open MemberVault or accept private-domain objects.
- Full PAN appears only inside encrypted records and short-lived sensitive buffers.
- Sensitive access emits an audit event with account ID and purpose, never the secret value.
- Telemetry is off by default.
- Local accounting never depends on AI or registrar availability.

Phase 2A regression coverage: `crates/member-vault/tests/security_invariant.rs` proves end-to-end that after persisting an encrypted identity, no plaintext PAN appears in any persisted file, audit record, AI payload, or envelope serialization. `crates/identity-security/tests/logging.rs` proves validation errors never echo input.

## Accepted limitation

A trusted member with privileged control of an enrolled device can potentially extract credentials/data available to that device. A zero-server local-first design mitigates accidental and remote exposure but cannot defeat the authorized endpoint owner.
