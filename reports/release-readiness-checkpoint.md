# Sanket IPO — Release Readiness Checkpoint

**Authoritative checkpoint:** `/home/harshdev/HermesWorkspaces/MY_IPO/reports/release-readiness-checkpoint.md`

## Governance

- `FEATURE_FREEZE = ACTIVE`
- `FEATURE_EXPANSION_ALLOWED = false`
- `ZERO_INCREMENTAL_SPEND_REQUIRED = true`
- Kanban is the only workflow state authority. This file is the single project checkpoint; `reports/release-readiness/initial-state-evidence.md` is evidence only.
- No new product feature, provider, integration, analytics, dashboard, AI system, architecture layer, or aesthetic redesign may be created.
- No PAN, credential value, keyring value, external provider token/request, MUFG request, financial mutation, reset, or history rewrite occurred. The accepted source was frozen locally in release commit `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`; no push occurred.

## Current phase

- **Active phase:** `FINAL — Release Ready Decision`
- **Phase state:** `RELEASED_SCOPED / ASTRA_HIGH APPROVED`; R0, R1, R2, R3, R4, R5, and FINAL are evidence-backed DONE under their approved scopes. R4 is DONE for the closest supported Linux cross-build state; Windows native runtime acceptance remains unavailable and explicitly unclaimed.
- **Completed phase:** `R0 — State Recovery + Feature Freeze` (`DONE` with completion metadata).
- **Completed phases:** `R1 — Security Closure`, `R2 — Functional + Data Integrity Closure`, and `R3 — Linux Release Readiness` are DONE with ASTRA_HIGH approval artifacts.
- **Successor phases:** R4, R5, and FINAL are complete under scoped ASTRA_HIGH approvals; no feature-development successor exists.

## Repository state

- Root: `/home/harshdev/HermesWorkspaces/MY_IPO`
- Repository: `harshvardhandpu/MY_IPO`
- Branch: `feature/aether-ui-redesign`
- Release source commit: `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`
- Source snapshot SHA-256: `e9a9152b79d84233e179a08060e03d661cecfc47b5ee3608dd65515f229c5a53`
- The accepted source worktree was clean at freeze; the release manifest and this checkpoint record post-build release metadata only.
- `git diff --check`: PASS
- No push was created.

## Persistent Kanban

- Existing Sanket task system before setup: none found. The unrelated KINETIC board was not reused.
- Native board: `sanket-ipo-release-readiness`
- Display name: `Sanket IPO — Release Readiness`
- Persistent database: `/home/harshdev/.hermes/kanban/boards/sanket-ipo-release-readiness/kanban.db`
- Hermes project binding: `my-ipo` → this board; primary folder is the project root above.
- Native states used: `ready` = runnable, `todo` = dependency-waiting backlog, `blocked` = capability/security gate, `review` = implementation awaiting review, `done` = evidence-backed completion.
- Latest native board counts: `blocked=1`, `done=12`, `todo=0`, with no `ready` or `running` cards.
- The root card has no parent/child dependency edge and therefore cannot block runnable children.

## Master root card

- `SANKET-ROOT — Sanket IPO — Release Readiness Program`
- Native status: `BLOCKED` with capability reason because it is governance-only and must never dispatch.
- It tracks freeze policy, phase state, current HEAD, checkpoint, blockers, Linux/Windows readiness, and final authority. It is not an implementation prerequisite.

## Minimal roadmap

1. **R0 — State Recovery + Feature Freeze:** DONE.
2. **R1 — Security Closure:** DONE; ASTRA_HIGH approved.
3. **R2 — Functional + Data Integrity Closure:** DONE; ASTRA_HIGH approved.
4. **R3 — Linux Release Readiness:** DONE; ASTRA_HIGH approved.
5. **R4 — Windows Release Readiness:** DONE for the ASTRA_HIGH-approved cross-built artifact state; native runtime validation remains pending.
6. **R5 — Cross-Platform Acceptance:** DONE as scoped ASTRA_HIGH-approved acceptance; Windows native runtime remains pending.
7. **FINAL — Release Ready Decision:** DONE; ASTRA_HIGH approved the platform-scoped release.

## Cards created

Required roadmap cards:

- `SANKET-ROOT` — blocked governance root
- `SANKET-R0` — done
- `SANKET-R1` — done; ASTRA_HIGH-approved
- `SANKET-R2` — done; ASTRA_HIGH-approved
- `SANKET-R3` — done; ASTRA_HIGH-approved
- `SANKET-R4` — done; ASTRA_HIGH-approved cross-built Windows artifact with native-runtime limitation
- `SANKET-R4-RECOVERY` — bounded native Windows route audit completed; no zero-incremental-spend native route was available
- `SANKET-R5` — done; ASTRA_HIGH-approved scoped cross-platform acceptance
- `SANKET-FINAL` — done; ASTRA_HIGH-approved platform-scoped release

Verified blocker cards only:

- `TEST-SEC-001` — done: production no-keyring denial test is hermetic without weakening production keyring enforcement; targeted and full Rust evidence passed.
- `TEST-UI-001` — done: Biome Tailwind directive parsing and formatter-only fixes restored the frontend check gate without adding UI scope.
- `SEC-AUTH-01` — native status `DONE`; `APPROVE_MINIMAL_DESIGN` remains the accepted design authority decision and is not reopened without invalidating evidence.
- `BUG-AUTH-01` — native status `DONE`; direct ASTRA_HIGH approval recorded after remediation and evidence review.

No speculative future cards were created.

## Dependency graph summary

- `TEST-SEC-001 → SANKET-R1`
- `SEC-AUTH-01 → SANKET-R1`
- `SANKET-R1 → SANKET-R2` — satisfied
- `TEST-UI-001 → SANKET-R2`
- `SEC-AUTH-01 → BUG-AUTH-01 → SANKET-R2` — satisfied
- `SANKET-R2 → SANKET-R3` — satisfied
- `SANKET-R2 → SANKET-R4` — satisfied
- `SANKET-R3 → SANKET-R4` — satisfied; cross-build scope accepted with native runtime limitation
- `SANKET-R3 → SANKET-R5` — satisfied
- `SANKET-R4 → SANKET-R5` — satisfied
- `SANKET-R5 → SANKET-FINAL` — satisfied; FINAL approved scoped release
- `SANKET-ROOT` has no blocking edge.

## Security gate status

**PASS for R1/R2 security and functional closure; not release-approved overall.** Existing security design is present and preserved: encrypted sensitive identity boundary, purpose-scoped audit, explicit application/provider-scoped expiring one-shot lookup authorization, OS-keyring production gate, fail-closed provider status normalization, and no AI/private-data path. Fresh evidence:

- `npm run secrets`: PASS — 210 files checked.
- `cargo test -p sanket-identity-security --test password_credentials`: PASS — 5 tests; Argon2id verifier primitive.
- `cargo test -p sanket-domain --test auth_events`: PASS — 3 tests; authentication event metadata only.
- `cargo test -p sanket-member-vault --test auth_initialization`: PASS — 2 tests; vault-backed initialization only.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- Focused authorization coverage is present in the existing Rust test suite.
- Full `cargo test --workspace`: PASS — the existing isolated `UnavailableKeyProvider` seam makes `production_without_keyring_denies_lookup_authorization` deterministic while preserving production fail-closed behavior. This closes `TEST-SEC-001`; it is not a production security bypass.
- Authentication is verified end-to-end at code/test level and was approved by ASTRA_HIGH in `reports/release-readiness/bug-auth-01-astra-high-approved.json`; event/vault authority, Argon2id credentials, invite/signup/approval/revocation, generic login/backoff, native sessions/logout, Tauri protected-command guards, authenticated onboarding identity binding, redacted auth DTO debug/wire output, and the minimal React auth gate all pass targeted/full Rust/frontend/security gates. Platform install and clean-profile smoke evidence remain open.
- No live provider/MUFG request and no PAN access occurred.

### SEC-AUTH-01 authority policy

- `SEC-AUTH-01` is a mandatory `R1` release gate, not a dormant human-wait state. Its design decision is already accepted and must not be reopened without invalidating evidence.
- Astra is the standing project authority. `ASTRA_MEDIUM` is invoked automatically for routine engineering/evidence decisions; `ASTRA_HIGH` is invoked automatically for security architecture, trust-boundary, authentication, credential/storage, phase-closure, release, production-readiness, governance, and final-release decisions. Medium decisions that exceed scope escalate automatically to High.
- Required decision scope: login; private/invite-controlled signup/account creation; logout; session persistence/recovery; failed-login handling; secure credential storage; no plaintext passwords; no credential leakage; no auth bypass through SQLite/config edits; Linux support; Windows support.
- Supabase or another mandatory cloud database is prohibited unless the authority review proves it is strictly required for an existing release requirement.
- After `APPROVE_MINIMAL_DESIGN`, `BUG-AUTH-01` activates automatically; only the minimum approved flow may be implemented, then tested and submitted automatically to `ASTRA_HIGH`. `REVISE` triggers bounded remediation, retest, and resubmission. `FUNDAMENTAL_BLOCKED` blocks only the affected chain while unrelated safe work continues.
- Authority decision: `APPROVE_MINIMAL_DESIGN`; `backend_required=false`; decision artifact `reports/release-readiness/sec-auth-01-authority-decision.json`; review artifact `reports/release-readiness/sec-auth-01-authority-review.md`. This is design approval only, not implementation or release approval.
- Decision hashes: authority JSON `bc3880d7082cf9c025b2c255b66b2ccf6966df0d8bdb46412941d3d0f2c362c0`; review artifact `acde8c14b94ef072b6b3d53758cc2c55e80a0fef443a552fcd6247a564507db8`.
- Automatic post-approval transition: native readback confirms `SEC-AUTH-01=done`, `BUG-AUTH-01=done`, `SANKET-R1=done`, `SANKET-R2=done`, and `SANKET-R3=done`; R4 is now natively BLOCKED on the recorded Windows runtime capability gap, and no R4 authority decision is claimed.

### Astra autonomous continuity policy

- Durable policy artifact: `reports/release-readiness/astra-governance-policy.md`.
- Policy SHA-256: `83e4269506b9398339fe355b2a49fedfc20a138d24179e30fe6cf6536f956245`.
- Authority lifecycle: `RUNNING → REVIEW → ASTRA_DECISION → APPROVED / REVISE / FUNDAMENTAL_BLOCKED`.
- `BLOCKED` is reserved for typed dependency, capability, or true human-only exceptions; it is never a generic “waiting for Astra” state.
- The single autopilot continues until `SANKET-FINAL = RELEASE READY APPROVED` or a true human-only exception. Worker failure, provider failure, model disagreement, security review, phase closure, and Astra unavailability are not human-only exceptions.
- True human-only exceptions: interactive authentication, account-holder action, information only the owner possesses, explicit secret/key entry, legal/account ownership, an irreversible user-data action explicitly reserved to the owner, or a direct governance-policy change.

## Functional and data-integrity status

**CLOSED for R2.** ASTRA_HIGH approved `reports/release-readiness/r2-functional-data-integrity-evidence.md` with no blockers. Frontend tests, Python secret tests, frontend check/format, typecheck, production build, Rust workspace/auth boundaries, event-authoritative replay/idempotency, VOIDED/duplicate semantics, durable recovery, and persistence/security evidence passed. Linux v1 is separately finalized and ready; full cross-platform acceptance is not claimed while Windows validation remains open.

## Linux readiness status

**CLOSED — ASTRA_HIGH APPROVED (R3 historical approval).** One approved build was run from source snapshot `f6554fc7c7228e8f27a26f16a8a75956732acfdf64275ca0406b6daf07c3f87`; all seven source identifiers matched after build. That approved DEB hashes to `3cb2130d55db9796f55a4a0c75865af333de13dfb1ce8f9401b3e733ca405b47` (6,551,632 bytes), and its extracted payload binary hashes to `a6d80bd8b954ef4e9fca1b7e826592b6552fcf08fd5419d06bb10b09f9aeb4b0` (20,236,048 bytes). The unbundled target hash is `f366d8fdc052c9e1d1cad6b76cfcbe6c8e7e90520c8976b910331334b32d15f7` with `UNK` marker; the packaged payload has `DEB` marker, same ELF Build ID/size, and is the approved release binary. The final frozen-source artifact is recorded separately in `## Finalization status` and `reports/release-readiness/release-manifest.md`. ASTRA_HIGH approval artifact: `reports/release-readiness/r3-astra-high-approved.json`. Development synthetic mode remains the default when `SANKET_SECURITY_MODE` is unset; release packaging proves secure mode explicitly.

## Finalization status

**SANKET IPO LINUX V1 READY FOR USE.**

### Visible program status

- `LINUX_V1 = RELEASED / READY FOR USE`
- `WINDOWS_V1 = INSTALLER AVAILABLE / CROSS-BUILT ON LINUX / NATIVE RUNTIME VALIDATION PENDING`
- `CROSS_PLATFORM = APPROVED_SCOPED / PENDING_WINDOWS_NATIVE_VALIDATION`
- `SANKET-R3 = DONE`
- `SANKET-R4 = DONE (CROSS-BUILT STATE)`
- `SANKET-R5 = DONE (SCOPED ACCEPTANCE)`
- `SANKET-FINAL = DONE (SCOPED RELEASE)`

The Windows native-runtime limitation is platform-scoped and does not make Linux
v1 unusable or unreleased. The approved Windows state is cross-built installer
availability, not native runtime readiness. Final platform-scoped release approval
is recorded; no repeated recovery work is authorized while native Windows capability
is unchanged.

- `LINUX_V1_READY = YES`
- Manifest: `reports/release-readiness/release-manifest.md`
- Final source commit: `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`
- Final DEB: `target/release/bundle/deb/Sanket IPO_0.1.0_amd64.deb`
- Final DEB SHA-256: `0d823dce7cb003a003792673e93cf0e1b607794ee40e4890e6593c8df5035e08`
- Final packaged binary SHA-256: `a6d80bd8b954ef4e9fca1b7e826592b6552fcf08fd5419d06bb10b09f9aeb4b0`
- Final source and lockfile hashes, build command, install/start/restart smoke result, and spend status are recorded in the manifest.
- R3 remains DONE and ASTRA_HIGH-approved; it was not reopened.

**Windows remains `NATIVE RUNTIME VALIDATION PENDING`.** R4 is DONE for the approved cross-built artifact state, while no Windows-native-ready claim is made.

## Windows readiness status

**CROSS-BUILT ARTIFACT AVAILABLE — NATIVE RUNTIME PENDING.** R4 was approved by
ASTRA_HIGH using the Tauri/cargo-xwin `x86_64-pc-windows-msvc` NSIS path. The
standalone PE32+ AMD64 application hashes to
`1c11bc1d1fbbdefbbab184cc3a3376748d63c218ee5f94b7c4e44e818f9c9882` and the NSIS
installer hashes to
`5ec68f092e6fc0606df17cfd387f5e258896be13c79eda195020875c0ea886dc`. The extracted
installer payload hash matches the standalone application. The NSIS bootstrap stub
is PE32 i386; the bundled application payload is AMD64. R4 and R5 authority artifacts
are `reports/release-readiness/r4-astra-high-cross-build-approved.json` and
`reports/release-readiness/r5-astra-high-cross-platform-approved.json`.

Native Windows Credential Manager behavior, Windows data paths/ACLs, first
launch/auth, restart/persistence, clean-profile installation, WebView2, secure
mode, uninstall/data preservation, and runtime smoke testing remain unverified.
`WINDOWS_NATIVE_RUNTIME_VALIDATED = false`. Wine is not used as evidence.

## Acceptance results

| Gate | Status | Evidence |
|---|---|---|
| Security | PASS for R1 | ASTRA_HIGH-approved R1 and BUG-AUTH-01 evidence; platform security runtime evidence remains open |
| Existing functional flows | PASS for R2 local scope | ASTRA_HIGH-approved R2 evidence; platform smoke remains open |
| Data integrity | PASS for R2 | ASTRA_HIGH-approved event/projection/recovery evidence |
| Frontend tests | PASS | `npm run test` |
| Frontend check/format | PASS | `npm run check`, `npm run format:check`; Tailwind directive parsing enabled and formatter-only fixes applied |
| Rust tests | PASS | `cargo test --workspace` after hermetic no-keyring test verification |
| Linux install/smoke | PASS | Final frozen-source DEB/payload hashes and disposable install/start/restart smoke are recorded in the release manifest |
| Windows install/smoke | CROSS-BUILT / NOT NATIVE-VALIDATED | R4/R5 approved artifact and payload evidence; native Windows runtime remains pending |
| Restart/persistence | PASS for Linux / PENDING for Windows | Final Linux disposable profile restart/persistence passed; Windows runtime evidence remains open |
| Secret scan | PASS | `npm run secrets` |
| Critical/high blockers | WINDOWS NATIVE RUNTIME ONLY | Linux V1 is READY; Windows installer is available, while native Windows runtime validation remains pending |
| Zero incremental spend | PASS | final Linux build used the authorized local toolchain; no paid provider or fallback used |
| Final authority review | APPROVED (SCOPED) | ASTRA_HIGH final platform-scoped approval; native Windows runtime remains pending |

## Artifact provenance and hashes

| Artifact path | Timestamp | Purpose/card | Producer/input | Result | SHA-256 |
|---|---|---|---|---|---|
| `reports/release-readiness/initial-state-evidence.md` | 2026-09-07T11:04:16+05:30 | R0 state recovery | Hermes Agent terminal; current Linux worktree | recovered state and fresh gates | `db11c42465645a762c5da8781936302bb601d1887cbee68d80c1cda186be14d2` |
| `docs/security/THREAT_MODEL.md` | setup readback | R1 security context | repository source | inspected | `b6e8d3388ce40afb9812d2b9ddfa8f9c26a82cfc00e2c53aa60746e06e7974fb` |
| `docs/security/CONTROLLED_REAL_PAN_PILOT.md` | setup readback | R1 security context | repository source | inspected | `dc0186e5cb454e646b86151566c414506811eafeb616e670d9b0cef305f6291f` |
| `docs/architecture/ARCHITECTURE.md` | setup readback | R0 architecture | repository source | inspected | `c7371025914c1da8d5a09dfe1e8525b896b12db09d7ca86e4b5438bb02b8637e` |
| `docs/handoffs/CURRENT_STATE.md` | setup readback | R0 state context | repository source | inspected | `3f6374ecf7fd533b87563cf43a4b64199d31ca0b2bb888961819e43fe743901b` |
| `package.json` | setup readback | R0/R2 toolchain | repository source | inspected | `b20d3a8173dd09eacbfa41bdd35f3c5ea3b102bbf7250b8416c4c5de15ad0b2c` |
| `apps/desktop/src-tauri/tauri.conf.json` | setup readback | R3/R4 packaging | repository source | inspected | `b14e459630f3e3e09cffa9e2e4b1694fb3f2bb9bf1d4fbb334494afd4f64344f` |
| `reports/release-readiness/r3-linux-release-provenance.json` | fresh exact build | SANKET-R3 | source snapshot, one-build record, artifact/payload hashes | internally consistent; ASTRA_HIGH approved | `40861cfa97a0222535d79c5522d0bd3775849b551ae2883d6d0c1bb3ce484e97` |
| `reports/release-readiness/r3-linux-release-evidence.md` | fresh exact build | SANKET-R3 | corrected Linux package/evidence report | final DEB `3cb213…`; payload `a6d80b…` | `ba36cb58fe0a520486b447430ca6b3335f25fd986662ff5f8759c2e47e733d5a` |
| `reports/release-readiness/r3-astra-high-approved.json` | 2026-09-07T15:47:06+05:30 | SANKET-R3 | ASTRA_HIGH phase closure | `APPROVED`, no blockers | `31ccfba222463dcdcb8e3bf8969d2a7e44fa344a4473a8cc981122a4c17c4d49` |
| `reports/release-readiness/r4-windows-cross-build-evidence.md` | 2026-09-08 | SANKET-R4 | Linux Tauri/cargo-xwin MSVC NSIS cross-build | ASTRA_HIGH-approved cross-built artifact; native runtime false | `ba0989b046a7e21ecc6277b4562b82472610f3bb55f4226fa3ef7b81fe0258f0` |
| `reports/release-readiness/r4-native-windows-recovery.md` | 2026-09-07T17:34:41+05:30 | SANKET-R4-RECOVERY | native Windows route audit | no valid zero-incremental-spend native route; R4 capability block preserved | `2d6ad70556cf54703a94283551569541557c08b06d4861f85b46bd7f846aa189` |
| `reports/release-readiness/r4-astra-high-cross-build-approved.json` | 2026-09-08 | SANKET-R4 | ASTRA_HIGH authority | `APPROVE_CROSS_BUILT_WINDOWS_ARTIFACT`; no blockers | `78d232573af8f5541c27ec63bd50da340462f11a191ccc7a6838a220364c3ce3` |
| `reports/release-readiness/r5-cross-platform-acceptance.md` | 2026-09-08 | SANKET-R5 | Linux native + Windows cross-built acceptance | ASTRA_HIGH-approved scoped cross-platform state | `140a82df075a4bf7144bece9a0dd040459a8754dccd0a9e61c2aa4d546151217` |
| `reports/release-readiness/r5-astra-high-cross-platform-approved.json` | 2026-09-08 | SANKET-R5 | ASTRA_HIGH authority | `APPROVE_CROSS_PLATFORM_SCOPED`; no blockers | `8dc43c098d306dabd0299bf24515ba0100e72b0f3211ba03fa90eedd2b2a56b0` |
| `reports/release-readiness/final-astra-high-platform-scoped-approved.json` | 2026-09-08 | SANKET-FINAL | ASTRA_HIGH final authority | `APPROVE_PLATFORM_SCOPED_RELEASE`; no blockers | `de4bfbe136384f958698963f54ce48a1530bf160f664527987c3e00b5e98a6a3` |
| `reports/release-readiness/release-manifest.md` | 2026-09-08 | final release metadata | platform-scoped manifest | Linux native validated; Windows cross-built installer; native runtime pending | `2214e69e7b51746ce9ddc203c4c34df50e56afe01ac17671157dea8c5ca6dd7e` |
| `reports/release-readiness/sec-auth-01-authority-decision.json` | 2026-09-07T11:46:46+05:30 | SEC-AUTH-01 | SECURITY_AUTHORITY design decision | `APPROVE_MINIMAL_DESIGN`, backend not required | `bc3880d7082cf9c025b2c255b66b2ccf6966df0d8bdb46412941d3d0f2c362c0` |
| `reports/release-readiness/sec-auth-01-authority-review.md` | 2026-09-07T11:46:46+05:30 | SEC-AUTH-01 | Hermes authority invocation | approved design; implementation later returned to R1 review | `acde8c14b94ef072b6b3d53758cc2c55e80a0fef443a552fcd6247a564507db8` |
| `reports/release-readiness/bug-auth-01-astra-high-approved.json` | 2026-09-07T13:51:17+05:30 | BUG-AUTH-01 | ASTRA_HIGH implementation approval | `APPROVED`, no blockers | `bd31435ac025a1d820b4e9620f9b8b21345c117557b33918ec12d1b2b1e72896` |
| `reports/release-readiness/r1-astra-high-approved.json` | 2026-09-07T14:02:43+05:30 | SANKET-R1 | ASTRA_HIGH phase closure | `APPROVED`, no blockers | `14947dea7c447b7f45220d67cb3fc5d0659128f628cf4817bd6281327e62ccdb` |
| `reports/release-readiness/r2-astra-high-approved.json` | 2026-09-07T14:06:30+05:30 | SANKET-R2 | ASTRA_HIGH phase closure | `APPROVED`, no blockers | `0ca5a91eb24863245a82fb95e1493f71bf1df0c420acfb54c494a8ec7d6e6756` |
| `reports/release-readiness/astra-governance-policy.md` | 2026-09-07T13:34:00+05:30 | project governance | ASTRA authority and continuity policy | automatic routing/escalation and non-passive Kanban lifecycle | `83e4269506b9398339fe355b2a49fedfc20a138d24179e30fe6cf6536f956245` |
| `apps/desktop/src-tauri/src/auth.rs` | 2026-09-07T13:33:39+05:30 | BUG-AUTH-01 | native AuthService | event-backed accounts/invites/approval/revocation, Argon2id, generic login/backoff, sessions/logout, transient-secret redaction | `17af8871620b286a450da2984a001989441e44c47ecf5494a6cebfcf6bd686e4` |
| `apps/desktop/src-tauri/src/lib.rs` | 2026-09-07T13:44:54+05:30 | BUG-AUTH-01 | Tauri boundary | auth commands, native session ownership, protected-command actor normalization, onboarding account binding | `9e46638f67ad44942c3f9bbafe917ecf0245ca0d7f249e6bd21a1ca68a92314d` |
| `crates/identity-security/src/password.rs` | 2026-09-07T13:20:02+05:30 | BUG-AUTH-01 | credential primitive | pinned Argon2id PHC hashing/verification with zeroized transient input | `c456d1f4a0f3183fb07a88ee5106f845764ff0828564dcf1449ec388c74c7891` |
| `apps/desktop/src/App.tsx` | 2026-09-07T13:20:02+05:30 | BUG-AUTH-01 | minimal UI auth gate | login/bootstrap/invite signup gate and existing-shell logout | `9d2a444afddb217d7e30687f1a692d82733c5ded23ff9df202a980a743324958` |
| `biome.json` | fresh verification | TEST-UI-001 | Biome configuration | Tailwind directive parser enabled | `781d6fec5ce4e0644e070f486a5354483c16c862f78f6e1d4b380c81095ef98f` |
| `apps/desktop/src/App.tsx` | fresh verification | TEST-UI-001 | Biome formatter | formatter-only normalization; auth/product behavior unchanged | `eac4202e12af726514a8661435491c17d45c3deaad9dcd8effae26116835c298` |
| `apps/desktop/src/components/ui/button.tsx` | fresh verification | TEST-UI-001 | Biome formatter | formatter-only normalization | `21379a0a77331b17580e57cb81c9cf8dc436f50667d57ef3e88ee7506bc3695f` |
| `apps/desktop/src/index.css` | fresh verification | TEST-UI-001 | Biome formatter | formatter-only normalization with Tailwind syntax accepted | `b9347cab3cb6a77770316e4301aab4c77eba181e3f08a983550c745c72b28316` |
| `apps/desktop/src/styles.css` | fresh verification | TEST-UI-001 | Biome formatter | formatter-only normalization | `430b9b34c68f3f53424b11e4461f8ad66205f16ac2d13597fbdab5adfc86434b` |

The checkpoint’s own SHA-256 is captured by the final fresh readback command rather than embedded recursively.

## Autopilot

- Existing unrelated job `kinetic-autopilot` was not reused.
- Sanket job: `sanket-ipo-release-autopilot` (`26ff3f32fa7d`)
- Schedule: every 30 minutes, forever; local-only delivery; currently PAUSED after scoped finalization.
- Workdir: `/home/harshdev/HermesWorkspaces/MY_IPO`
- Board: `sanket-ipo-release-readiness`
- Tool scope: unrestricted agent toolset for board/checkpoint reconciliation, worker routing, and authority invocation; no provider/MCP spend route unless explicitly authorized by the project policy.
- Policy after scoped finalization: remain quiet; do not create cards, dispatch workers, rerun the completed route audit, run builds/tests, or manufacture work. Reactivate the existing recovery only when new evidence appears of an available native Windows host, verified free CI route, or authorized connected Windows runner. No secrets/PAN/live providers, no paid route, no worker self-approval, and no push.

## Human-only exceptions

No human action is required for routine engineering, tests, packaging repair, or card management. Human-only exceptions remain limited to interactive authentication/account-holder action, information only the owner possesses, legal action, irreversible user-data operations explicitly reserved to the owner, explicit secret/key entry, and governance-policy changes. None was requested or performed during setup.

## Important decisions

- Feature freeze is active and applies to the UI and all integrations.
- Existing local-first event/vault authority and SQLite projection policy is preserved.
- Supabase/cloud database work is post-release/optional and was not introduced.
- Authentication cards were created because the repository’s own required-authentication section is not implemented in the live app; they are release repair, not feature expansion.
- `TEST-SEC-001` was a test/environment defect; its existing isolated provider seam is verified and production keyring enforcement remains fail-closed.
- The accepted UI/auth/security source was frozen without functional changes in release commit `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`.

## Latest autonomous verification

- `TEST-SEC-001` was claimed and completed through native Kanban. Fresh readback confirms `done` with completion metadata; `SANKET-R1`, `SANKET-R2`, and `BUG-AUTH-01` remain evidence-backed `done`; root remains governance-only and blocked.
- `TEST-UI-001` was claimed and completed through native Kanban. Fresh-process readback confirms `done` with completion metadata; the Sanket board is active and persists at the recorded SQLite path.
- Fresh implementation evidence: `cargo test -p sanket-desktop --lib` PASS (26 tests); `cargo test --workspace` PASS; `cargo clippy --workspace --all-targets -- -D warnings` PASS; `cargo fmt --all -- --check` PASS; password tests PASS (5), domain auth tests PASS (3), vault initialization tests PASS (2); `npm run check` PASS (secrets, format, typecheck, 10 frontend tests, 4 Python tests; 211 files scanned); `npm run build` PASS; `git diff --check` PASS. The focused auth DTO regression proves transient passwords/invite secrets do not appear in Debug output and session tokens are omitted from the serialized login response. Authenticated onboarding identity binding is covered by a native regression test. `npm run lint` remains FAIL on pre-existing repository-wide diagnostics outside TEST-UI-001 scope.
- Fresh R4 evidence: `cargo xwin check --workspace --target x86_64-pc-windows-msvc`, Tauri production frontend build, optimized Windows release build, generated NSIS script/package, 7-Zip extraction, AMD64 payload verification, secret scan, production dev-server-marker scan, and `git diff --check` passed. Current artifact hashes and authority evidence are recorded above. Native Windows runtime acceptance remains unavailable; Wine is not accepted as a substitute.
- Fresh R4 recovery: `SANKET-R4-RECOVERY` (`t_91d31dff`) audited the existing native host, local runner inventory, GitHub Actions `windows-latest`, connected CI, and configured Windows hosts. No native Windows host/self-hosted runner exists; GitHub Actions is private-repository usage with unverified billing allowance and no `workflow_dispatch`, while the accepted source was not on the remote branch at audit time. Recovery artifact SHA-256 is `2d6ad70556cf54703a94283551569541557c08b06d4861f85b46bd7f846aa189`. No paid route, interactive authentication, or secret access was used.
- Fresh native board readback after finalization: `SANKET-R3=done`, `SANKET-R4=done`, `SANKET-R5=done`, and `SANKET-FINAL=done`; R3 remains native-approved, R4 is cross-build-approved, R5 is scoped cross-platform-approved, and FINAL is platform-scoped ASTRA_HIGH-approved. No push occurred.
- The accepted implementation is frozen in release commit `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`; release metadata is recorded in `reports/release-readiness/release-manifest.md`. No push was performed.
- No PAN, credential value, keyring value, external provider/MUFG token/request, or financial mutation occurred; synthetic test session tokens were ephemeral and not retained.

## Next legitimate action

- Scoped finalization is complete: `SANKET IPO LINUX V1 READY FOR USE`; the Windows x86_64 MSVC NSIS installer is available cross-built from Linux; R4, R5, and FINAL are ASTRA_HIGH-approved under their scoped policies. `WINDOWS_NATIVE_RUNTIME_VALIDATED = false` remains authoritative. Keep the existing autopilot paused and quiet; do not recreate the recovery card or repeat the route search unless genuinely new native Windows capability evidence appears.
