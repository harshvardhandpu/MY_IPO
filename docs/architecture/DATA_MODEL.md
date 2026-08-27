# Data Model

All IDs are opaque UUIDv7/ULID-style stable identifiers. Money is integer paise; percentages are integer basis points. Timestamps are RFC 3339 UTC.

## Identity

- `CoreMember`: id, display name, role, primary account id, sensitive record id, masked PAN, consent metadata, status.
- `FriendAccount`: id, owner member id, display label, sensitive record id, masked PAN, broker metadata, share eligibility/basis points, status.
- `SensitiveIdentityEnvelope`: envelope version, key id, nonce, ciphertext, authenticated metadata. Full PAN exists only inside ciphertext. Persisted at `MemberVault/_secure_identity/<member-id>.enc` and `MemberVault/_secure_identity/friends/<friend-id>.enc` (atomic owner-only writes). See `docs/architecture/SENSITIVE_IDENTITY.md`.
- `Device`: id, member id, public key, status, created/revoked timestamps.

## Investments

- `InvestmentSession`: id, actor/member, declared capital paise, status, timestamps.
- `IPO`: id, typed/canonical name, issue metadata, registrar mapping, public provenance.
- `IPOApplication`: id, session id, IPO id, member id, status, expected allotment metadata.
- `InvestmentAllocation`: id, application id, account id/type, amount paise, lots/shares, friend-share eligibility and basis points.

## Proof and accounting

- `Proof`: id, related entity, kind, encrypted blob hash/path, safe display filename, content type, uploaded by/at.
- `Allotment`: allocation id, status, lots/shares/amount, source, provenance, recorded at.
- `ProfitCalculation`: allocation id, proceeds/cost/charges/gross/share/net paise, share basis points, price basis.
- `FriendSharePayment`: share id, due/paid amount, state, proof reference, timestamps.

## Automation

- `AllotmentCheckJob`: IPO/application scope, durable job state, attempt counters, retry schedule.
- `AllotmentCheckResult`: account id, provider, normalized status, quantities, safe metadata, provenance. Never PAN.
- `AllotmentReportCard`: projection joining safe display identity with normalized results and explicit profit basis.

## Projection rule

SQLite tables are rebuildable projections of valid durable events. The live SQLite file is excluded from Git and is never authoritative.
