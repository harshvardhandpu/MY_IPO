# SANKET-R4 — Windows Cross-Build Evidence

- **Recorded:** 2026-09-08
- **Card:** `t_5ca8e730` (`SANKET-R4`)
- **Mode:** `WINDOWS CROSS-BUILD MODE`
- **Scope:** Linux-to-Windows Tauri production cross-build and Linux-observable artifact verification only. No feature, redesign, provider, backend, architecture, signing, or native-host work.

## Required result semantics

- `WINDOWS_INSTALLER_CROSS_BUILT = true`
- `WINDOWS_NATIVE_RUNTIME_VALIDATED = false`
- `R4_NATIVE_WINDOWS_LIMITATION`: Windows installer was cross-built on Linux using the Tauri/cargo-xwin NSIS path. Native Windows install/runtime smoke testing has not been performed.
- Wine was not used as evidence.
- No paid CI, paid signing, MSI/WiX, provider request, PAN, credential value, or financial mutation occurred.

## Source provenance

- Repository: `/home/harshdev/HermesWorkspaces/MY_IPO`
- Branch: `feature/aether-ui-redesign`
- Build HEAD: `e2a17b8d2115b200bf0262ff03fd0b44364e1518`
- Worktree before and after build: clean
- Git diff check: PASS
- Current HEAD source archive SHA-256: `f39787381b965dc01673ce2cf1d272a7ee6bd582507dc1bb7d6186f96b4f0321`
- `package-lock.json` SHA-256: `1e555f631520fd462d4f17a79a97306547502d071c5259a4998f47190bc3b87d`
- `Cargo.lock` SHA-256: `fccd660464a349ba0d8379060f86db7d7791b29faaefd89b56492cbc4ce37422`
- The accepted Linux product source is release commit `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`. `git diff 6afe4d75..HEAD` contains only `reports/release-readiness-checkpoint.md` and `reports/release-readiness/release-manifest.md`; no product source changed after the accepted Linux freeze.
- Linux artifact provenance remains the approved R3 record and is not reopened.

## Toolchain and target

- Target: `x86_64-pc-windows-msvc`
- Node: `v26.8.1`
- npm: `12.0.2`
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Cargo: `cargo 1.96.0 (30a34c682 2026-05-25)`
- cargo-xwin: `cargo-xwin-xwin 0.23.1`
- Tauri CLI: `2.11.4`
- Clang: `22.1.8`
- LLD: `22.1.8`
- NSIS: `3.08-3+deb12u1` via a disposable local Debian tool image and a user-local `makensis` wrapper
- Rust target `x86_64-pc-windows-msvc`: installed

## Static Windows compatibility and preflight

- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: PASS.
- The preflight compiled `sanket-desktop`, authentication/session code, `keyring v3.6.3`, Tauri, and workspace crates for the MSVC target.
- `keyring` is configured with the `windows-native` feature.
- Windows app-data/path handling uses Tauri/platform directory APIs; no production-only Unix path code was found in `apps/desktop/src-tauri/src`.
- Unix permission extensions in shared crates are guarded by `cfg(unix)`.
- No compatibility repair to product source was required.

## Build

Requested command:

```text
npm run tauri:build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis --ci --no-sign
```

Observed result:

- Frontend production build: PASS.
- Rust `release` cross-compilation: PASS.
- Tauri produced the Windows release executable.
- The first NSIS invocation reached the generated Tauri script but the disposable wrapper did not mount Tauri's downloaded plugin cache. The bounded tooling repair mounted `/home/harshdev/.cache` read-only and preserved the caller working directory. The generated Tauri NSIS script then completed successfully without source changes or recompilation.
- The Tauri-generated intermediate `nsis-output.exe` was copied byte-for-byte to the expected Tauri bundle path; source and final installer hashes match.

## Windows executable

- Path: `target/x86_64-pc-windows-msvc/release/sanket-ipo.exe`
- Size: `15,137,792` bytes
- SHA-256: `1c11bc1d1fbbdefbbab184cc3a3376748d63c218ee5f94b7c4e44e818f9c9882`
- `file`: PE32+ executable for MS Windows, GUI, x86-64
- PE machine: `IMAGE_FILE_MACHINE_AMD64 (0x8664)`
- PE optional-header magic: `0x20B`
- Subsystem: Windows GUI
- Build profile: Cargo `release`

## NSIS installer

- Path: `target/x86_64-pc-windows-msvc/release/bundle/nsis/Sanket IPO_0.1.0_x64-setup.exe`
- Size: `4,106,322` bytes
- SHA-256: `5ec68f092e6fc0606df17cfd387f5e258896be13c79eda195020875c0ea886dc`
- Format: NSIS 3 Unicode self-extracting archive, LZMA-compressed
- Linux `file` identifies the NSIS bootstrap stub as PE32 i386. This is the Debian NSIS 3.08 bootstrap stub architecture; it is not the application payload architecture.
- The installer was extracted with 7-Zip. It contained 8 files, including `sanket-ipo.exe`.
- Extracted payload size: `15,137,792` bytes
- Extracted payload SHA-256: `1c11bc1d1fbbdefbbab184cc3a3376748d63c218ee5f94b7c4e44e818f9c9882`
- Extracted payload PE machine: `IMAGE_FILE_MACHINE_AMD64 (0x8664)`
- Extracted payload PE optional-header magic: `0x20B`
- The extracted payload hash exactly matches the standalone release executable.

## Release/security checks observable from Linux

- `npm run secrets`: PASS (`225` files checked).
- Binary string scans of the standalone executable, installer, and extracted executable: zero format-valid PANs, private-key markers, common live-token formats, JWT-like values, or API-key/secret assignment literals.
- Production frontend static output contains zero occurrences of `localhost`, `127.0.0.1`, `import.meta.hot`, `vite/client`, `__vite__`, or `VITE_DEV_SERVER_URL`.
- `git diff --check`: PASS.
- Post-build HEAD, branch, worktree status, source archive hash, lockfile hashes, and cached/uncached diff hashes matched the pre-build provenance.
- No native Windows install, first launch, Credential Manager, Windows ACL/data-path, WebView2, auth/session, restart/persistence, secure-mode, uninstall, or data-preservation test was run.

## Gate state before authority decision

- Cross-built Windows artifact: **AVAILABLE**.
- Linux-side artifact/provenance/security checks: **PASS**.
- Native Windows runtime validation: **PENDING / NOT PERFORMED**.
- R4 authority decision: **APPROVE_CROSS_BUILT_WINDOWS_ARTIFACT** by ASTRA_HIGH.
- Authority artifact: `reports/release-readiness/r4-astra-high-cross-build-approved.json`.
- Native runtime limitation remains authoritative: `WINDOWS_NATIVE_RUNTIME_VALIDATED = false`.

This report does not claim Windows native readiness. It is the evidence package for the closest supported cross-built R4 state requested by the project policy.
