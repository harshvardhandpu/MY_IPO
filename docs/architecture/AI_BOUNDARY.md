# AI Boundary

## Allowed inputs

External AI may receive public IPO data, filings, news, public market context, research papers, versioned owner algorithms, strategy/backtest artifacts, declared daily capital, planned amount per account, typed IPO names, and anonymous account count/slots.

## Forbidden inputs

PAN, UPI IDs, names, emails, phone numbers, proof bytes/paths, private member/account history, MemberVault paths/content, private profits, or private audit events.

## Structural enforcement

- `ai-gateway` depends on public intelligence/sanitized DTO crates only.
- It has no dependency on `member-vault`, `crypto` identity APIs, or allotment worker services.
- The request builder accepts `InvestmentDecisionPayload`/`IntelligenceContext`, never generic JSON or private domain objects.
- Tauri AI commands receive no MemberVault filesystem capability.
- Strategy Lab and News/Report services are constructed only with `IntelligenceVault`.
- CI dependency checks reject forbidden crate edges.

Phase 2A status: `InvestmentDecisionPayload` is implemented in `sanket-intelligence-vault` with a fail-closed `assert_safe()` validator that rejects PAN-like and UPI-like tokens in any string field. `Pan` does not implement `Serialize`, so private identity types cannot be serialized into AI payloads at compile time. Adversarial tests live in `crates/intelligence-vault/tests/ai_boundary.rs`.

## Sanitized decision contract

```json
{
  "session_id": "opaque",
  "declared_daily_capital_paise": 12000000,
  "account_count": 6,
  "ipos": [{"typed_name": "Example IPO Limited", "planned_amount_per_account_paise": 1500000}],
  "algorithm_version": "ipo-ranking-v001",
  "public_context_refs": []
}
```

The sanitizer uses an allowlist and constructs a new value field by field. It never removes forbidden fields from a generic private object.

## Required tests

Adversarial serialization tests must prove PAN, UPI, identity, proofs, and vault paths cannot enter an AI request. Source/dependency tests must prove AI/Strategy/News modules cannot open MemberVault.
