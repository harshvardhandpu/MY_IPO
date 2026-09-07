# Sanket IPO — Initial Release-Readiness Evidence

- Timestamp: 2026-09-07T11:04:16+05:30
- Card: SANKET-R0
- Producing tool: Hermes Agent terminal/read-only repository inspection
- Input/environment: `/home/harshdev/HermesWorkspaces/MY_IPO`, Linux, Node v26.8.1, npm 12.0.2, rustc/cargo 1.96.0, GDK 3.24.52, D-Bus 1.16.2
- Security boundary: no PAN, provider request, MUFG request, credential value, keyring value, or financial/application mutation accessed

## Repository state

- Branch: `feature/aether-ui-redesign`
- HEAD: `d3864557ee56282214733d44149cb2ed1fbc0ee6`
- Tracking: ahead 1 of `origin/feature/aether-ui-redesign`
- Worktree: dirty; six tracked files modified and eleven untracked frontend files
- `git diff --check`: PASS
- Remote: `origin` is `https://github.com/harshvardhandpu/MY_IPO.git`
- No commit, push, reset, or history rewrite performed

## Existing architecture recovered

- Tauri desktop application with React/Vite frontend.
- Append-only local event/vault state with SQLite local projection/query state.
- Sensitive identity access is mediated by the existing identity-security service and runtime security mode.
- Real-investor lookup uses explicit provider/application-scoped authorization and OS-keyring production gating.
- Allotment providers and fail-closed status normalization are covered by Rust tests.
- Upstox credentials use the existing OS-keyring credential store.
- Tauri bundle configuration declares all targets; no release installer artifact is currently tracked.
- Existing CI workflow: `.github/workflows/ci.yml`.

## Fresh verification

| Command | Result |
|---|---|
| `npm run secrets` | PASS — 201 files checked |
| `npm run typecheck` | PASS |
| `npm run test` | PASS — 2 frontend files / 10 tests; 4 Python tests |
| `npm run build` | PASS — Vite production build |
| `npm run check` | FAIL — four Biome errors; Tailwind `@theme` parsing disabled plus formatting errors in current UI files |
| `npm run format:check` | FAIL — same current UI formatting/parser issue |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | FAIL — one test: `production_without_keyring_denies_lookup_authorization` |
| isolated production/no-keyring test | FAIL reproducibly on this host; `security.real_pan_allowed` is true because the host exposes an active D-Bus/Secret Service environment |
| `git diff --check` | PASS |

The Rust failure is classified as a hermetic-test/environment defect, not evidence that production authorization is bypassed. The test assumes an unavailable keyring while the current host has a keyring backend; production code still requires `ProductionSecure`, `os-keyring`, and `real_pan_allowed` before real-investor lookup.

## Packaging inventory

- Tracked `.deb`: none
- Tracked `.rpm`: none
- Tracked `.AppImage`: none
- Tracked `.exe`: none
- Tracked `.msi`: none
- Windows packaging and clean-profile smoke test: not yet verified on a Windows runner
- Linux production package/install/smoke test: not yet verified

## Source evidence hashes

- `README.md`: `b357fb09d7280276f80ac5c6e3be6d6d9e2063d8770bfb500bc8120ad0402e61`
- `docs/handoffs/CURRENT_STATE.md`: `3f6374ecf7fd533b87563cf43a4b64199d31ca0b2bb888961819e43fe743901b`
- `docs/architecture/ARCHITECTURE.md`: `c7371025914c1da8d5a09dfe1e8525b896b12db09d7ca86e4b5438bb02b8637e`
- `docs/security/THREAT_MODEL.md`: `b6e8d3388ce40afb9812d2b9ddfa8f9c26a82cfc00e2c53aa60746e06e7974fb`
- `docs/security/CONTROLLED_REAL_PAN_PILOT.md`: `dc0186e5cb454e646b86151566c414506811eafeb616e670d9b0cef305f6291f`
- `package.json`: `b20d3a8173dd09eacbfa41bdd35f3c5ea3b102bbf7250b8416c4c5de15ad0b2c`
- `apps/desktop/src-tauri/tauri.conf.json`: `b14e459630f3e3e09cffa9e2e4b1694fb3f2bb9bf1d4fbb334494afd4f64344f`

This artifact is evidence only. It is not a second release checkpoint and contains no secrets or PAN.
