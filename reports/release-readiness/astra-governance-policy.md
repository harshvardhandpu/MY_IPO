# Sanket IPO — Astra Authority and Autonomous Continuity Policy

Effective: 2026-09-07
Board: `sanket-ipo-release-readiness`
Autopilot: `sanket-ipo-release-autopilot`

## Authority

Astra is the standing project authority. Authority invocation is part of autonomous execution; model-authority requirements are never passive human waits.

ASTRA_MEDIUM is the default authority for planning, implementation strategy, debugging, test strategy, remediation, dependency repair, provider/tool choice, worker/model choice, bounded refactoring, evidence evaluation, routine security review, routine release-readiness decisions, card creation/reuse, sequencing, recovery, and normal architecture interpretation.

ASTRA_HIGH is mandatory for security architecture, trust-boundary changes, authentication or credential/storage architecture, major architecture, irreversible data behavior, risk acceptance, phase closure or transition, Linux approval, Windows approval, cross-platform acceptance, production readiness, governance interpretation, and the final RELEASE READY decision.

A worker model is implementation-only. Workers may execute approved work, tests, debugging, documentation, packaging, and evidence collection. Workers may not impersonate Astra, approve their own High-authority architecture, weaken governance, close High-authority gates, or approve phase/release transitions.

## Automatic routing and escalation

For every authority-required card:

1. Move `RUNNING` to `REVIEW` after implementation/evidence collection.
2. Invoke ASTRA_MEDIUM for routine or medium-scoped decisions.
3. Invoke ASTRA_HIGH directly for every explicitly High-authority card.
4. If ASTRA_MEDIUM reports that the decision exceeds Medium scope, escalate automatically to ASTRA_HIGH.
5. If Astra is temporarily unavailable, retry within policy and preserve the card in `REVIEW` or a typed capability state. Never silently downgrade a High gate to a weaker model and never ask the user merely because Astra is unavailable.
6. Record the exact authority, model/provider route, decision artifact, hash, timestamp, and consequences on the card and checkpoint.

Valid outcomes are `APPROVED`, `REVISE`, and `FUNDAMENTAL_BLOCKED`.

`APPROVED`: close the authority gate, update the checkpoint, satisfy dependencies, activate the successor, and continue automatically.

`REVISE`: create the smallest bounded remediation, dispatch a qualified worker, run the required tests, collect fresh evidence, and return automatically to authority review.

`FUNDAMENTAL_BLOCKED`: block only the affected dependency chain with a typed capability/dependency reason, preserve evidence, and continue unrelated safe release work.

`BLOCKED` is never a generic “waiting for Astra” state. A model-authority requirement is not a human-only exception.

## Kanban state contract

Use native states semantically:

`RUNNING` means a worker is actively executing.

`REVIEW` means implementation/evidence is complete enough for authority review; it is not a passive block.

`BLOCKED` is reserved for a true typed dependency, capability, or human-only exception. A card awaiting Astra remains `REVIEW` while Astra is retried, unless the native board requires a typed capability block for a documented temporary outage.

`DONE` requires evidence-backed completion and the required authority decision.

Normal authority path:

`RUNNING → REVIEW → ASTRA_DECISION → APPROVED / REVISE / FUNDAMENTAL_BLOCKED`.

## Continuity

The single Sanket autopilot must reload the board, checkpoint, protected HEAD, and active evidence; recover interrupted work; claim READY tasks; dispatch qualified workers; run tests; collect evidence; invoke Astra; apply decisions; remediate REVISE; close verified gates; unlock successors; and continue until `SANKET-FINAL = RELEASE READY APPROVED` or a true human-only exception.

The autopilot must not ask “Should I continue?” when Astra has authority. Worker failure, provider failure, model disagreement, security review, phase closure, and temporary Astra unavailability are recovery/routing conditions, not human-only exceptions.

Do not create a competing tracker, reset the repository, push, deploy, access PAN/provider/keyring values, or weaken security tests.

## True human-only exceptions

Pause only for interactive authentication, account-holder action, information only the owner possesses, explicit secret/key entry, legal/account ownership action, an irreversible user-data operation explicitly reserved to the owner, or a direct change to project governance itself.

## Current authentication gate

`SEC-AUTH-01` is already approved with `APPROVE_MINIMAL_DESIGN`. Do not reopen it unless new evidence proves the approved design invalid.

`BUG-AUTH-01` continues under that approved design. When implementation/evidence is complete, submit automatically to ASTRA_HIGH. On `APPROVED`, close BUG-AUTH-01, update R1 evidence, satisfy dependencies, and continue. On `REVISE`, create bounded remediation, retest, and resubmit automatically. Do not wait for human confirmation.

## Evidence and fail-closed rules

Authority decisions never replace technical evidence. A card closes only after the required tests, artifact hashes, protected repository state, and native Kanban readback are verified. Preserve rejected decisions and failed attempts; do not copy them into the latest accepted authority state. Do not claim release readiness from a worker result, model name, heartbeat, or authority invocation alone.

Policy artifact SHA-256 is recorded in the durable checkpoint after each policy update.
