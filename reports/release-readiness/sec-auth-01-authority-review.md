# SEC-AUTH-01 — Security Authority Review

## Invocation record

- **Gate:** `SEC-AUTH-01`
- **Release phase:** `R1 — Security Closure`
- **Status:** `APPROVED_MINIMAL_DESIGN`
- **Invoked at:** `2026-09-07T11:46:46+05:30`
- **Authority role:** `SECURITY_AUTHORITY`
- **Authority model:** `gpt-5.6-luna`
- **Authority provider:** `openai-codex`
- **Delegation:** `deleg_e37c1a4f`
- **Subagent:** `sa-0-01ac7a38`
- **Read-only review:** yes
- **Identity note:** the active parent runtime identifies `gpt-5.6-luna` via `openai-codex`, and the authority response reports the same route; the delegation envelope did not independently populate its model field (`Model: ?`). No stronger authorized route was available in this session.
- **Decision:** `APPROVE_MINIMAL_DESIGN`
- **Backend required:** `false`
- **Decision artifact:** `reports/release-readiness/sec-auth-01-authority-decision.json`
- **Decision SHA-256:** `bc3880d7082cf9c025b2c255b66b2ccf6966df0d8bdb46412941d3d0f2c362c0`
- **Implementation status:** design approval only; authentication is not implemented or release-approved

## Mandatory decision scope

The authority must decide the smallest secure v1 authentication design that preserves the existing local-first architecture and covers:

- login
- private/invite-controlled signup/account creation
- logout
- session persistence/recovery
- failed-login handling
- secure credential storage
- no plaintext password storage
- no credential leakage
- no authentication bypass through SQLite/config edits
- Linux support
- Windows support

## Architecture constraints

- Append-only event/vault authority remains authoritative.
- SQLite remains projection/query state and must not be an authentication authority.
- Supabase or another mandatory cloud database is prohibited unless the authority proves it is strictly required for an existing release requirement.
- Existing PAN, OS-keyring, identity-security, provider, and allotment boundaries remain unchanged.
- No PAN, live provider request, credential value, keyring value, financial mutation, commit, or push is permitted during review.

## Required post-decision transition

- `APPROVE_MINIMAL_DESIGN`: activate `BUG-AUTH-01` automatically; implement only the approved minimum with test-first development; collect evidence; return to R1 review. This decision was returned and recorded.
- `BLOCKED` or `REJECT`: keep `SEC-AUTH-01` actively escalated with the exact machine-verifiable gap; do not wait for a human and do not implement authentication.
