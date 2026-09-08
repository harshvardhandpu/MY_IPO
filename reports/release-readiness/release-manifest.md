# Sanket IPO v1 Release Manifest

Generated: 2026-09-08

## Release status

**SANKET IPO v1**

**LINUX:** READY / NATIVE VALIDATED

**WINDOWS:** INSTALLER AVAILABLE / CROSS-BUILT ON LINUX

**WINDOWS_NATIVE_RUNTIME_VALIDATED = NO**

**SECURITY:** PASS

**AUTH:** PASS

**DATA INTEGRITY:** PASS

**ZERO-INCREMENTAL-SPEND:** PASS

**LINUX_V1_READY = YES**

The Linux release is ready for use. The Windows installer is available as a
Linux cross-built artifact, but native Windows install/runtime validation remains
pending and is not represented as complete by this manifest.

## Immutable source identity

- **SOURCE COMMIT:** `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`
- **SOURCE TREE:** `2e815f3304025cffc7a8fb6e3d1efeba4d0f6924`
- **SOURCE SNAPSHOT SHA-256:** `e9a9152b79d84233e179a08060e03d661cecfc47b5ee3608dd65515f229c5a53`
- **package-lock.json SHA-256:** `1e555f631520fd462d4f17a79a97306547502d071c5259a4998f47190bc3b87d`
- **Cargo.lock SHA-256:** `fccd660464a349ba0d8379060f86db7d7791b29faaefd89b56492cbc4ce37422`

The source snapshot is a deterministic `git archive` SHA-256 of the release
commit. No source files changed between the final gates, commit, and Linux build.

## Linux artifact

- **Version:** `0.1.0`
- **Architecture:** `amd64`
- **Build command:** `npm run tauri:build -- --bundles deb --ci --no-sign`
- **LINUX DEB:** `/home/harshdev/HermesWorkspaces/MY_IPO/target/release/bundle/deb/Sanket IPO_0.1.0_amd64.deb`
- **Workspace-relative DEB:** `target/release/bundle/deb/Sanket IPO_0.1.0_amd64.deb`
- **LINUX DEB SHA-256:** `0d823dce7cb003a003792673e93cf0e1b607794ee40e4890e6593c8df5035e08`
- **Packaged binary:** `/usr/bin/sanket-ipo`
- **Packaged binary SHA-256:** `a6d80bd8b954ef4e9fca1b7e826592b6552fcf08fd5419d06bb10b09f9aeb4b0`
- **Packaged binary size:** `20,236,048` bytes
- **ELF Build ID:** `e88d0ca4da198ff620c454f006d854722741f153`
- **Debian metadata:** package `sanket-ipo`, version `0.1.0`, architecture `amd64`

The packaged payload SHA-256 and ELF Build ID match the approved R3 Linux
payload. R3 remains closed; no concrete regression evidence reopened it.

## Windows cross-built artifact

- **Target:** `x86_64-pc-windows-msvc`
- **Build command:** `npm run tauri:build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis --ci --no-sign`
- **Windows application:** `/home/harshdev/HermesWorkspaces/MY_IPO/target/x86_64-pc-windows-msvc/release/sanket-ipo.exe`
- **Windows application SHA-256:** `1c11bc1d1fbbdefbbab184cc3a3376748d63c218ee5f94b7c4e44e818f9c9882`
- **Windows application size:** `15,137,792` bytes
- **Windows NSIS installer:** `/home/harshdev/HermesWorkspaces/MY_IPO/target/x86_64-pc-windows-msvc/release/bundle/nsis/Sanket IPO_0.1.0_x64-setup.exe`
- **Windows NSIS installer SHA-256:** `5ec68f092e6fc0606df17cfd387f5e258896be13c79eda195020875c0ea886dc`
- **Windows NSIS installer size:** `4,106,322` bytes
- **Extracted installer payload SHA-256:** `1c11bc1d1fbbdefbbab184cc3a3376748d63c218ee5f94b7c4e44e818f9c9882`
- **Payload architecture:** PE32+ AMD64 (`IMAGE_FILE_MACHINE_AMD64`, `0x8664`)
- **Installer stub note:** the Debian NSIS bootstrap stub is PE32 i386; the bundled Sanket payload is verified AMD64.
- **Cross-build evidence:** `reports/release-readiness/r4-windows-cross-build-evidence.md`
- **R4 authority:** `reports/release-readiness/r4-astra-high-cross-build-approved.json`
- **R5 authority:** `reports/release-readiness/r5-astra-high-cross-platform-approved.json`

The Windows installer was cross-built on Linux using the Tauri/cargo-xwin NSIS
path. Native Windows install/runtime smoke testing has not been performed.

## Verification record

Required final gates passed before the immutable source commit:

- Rust formatting check
- Focused desktop authentication tests
- Rust workspace tests
- Rust Clippy with warnings denied
- Frontend typecheck
- Frontend tests
- Frontend production build
- Secret scan: 226 files checked
- `git diff --check`

Final artifact verification passed:

- DEB metadata and artifact SHA-256 read back
- Package extracted and packaged executable SHA-256 read back
- ELF Build ID read back
- Disposable fakeroot DEB install passed
- Clean-profile first start passed
- Restart using the same isolated profile passed
- App-data files persisted under the isolated XDG data path with mode `600`
- Uninstall passed and the isolated user marker remained preserved
- No native Sanket process remained after the smoke run
- No provider, registrar, MUFG, real-investor, financial, or PAN operation occurred

Windows Linux-observable gates also passed: MSVC-target Rust preflight, release
cross-compilation, NSIS extraction, AMD64 payload verification, source/lockfile
provenance readback, secret scan, and production frontend dev-server-marker scan.
These checks do not establish native Windows runtime behavior.

## Windows runtime limitation

`WINDOWS_INSTALLER_CROSS_BUILT = true` and `WINDOWS_NATIVE_RUNTIME_VALIDATED = false`.
Native Windows execution, Credential Manager behavior, Windows app-data paths and
ACLs, clean-profile startup, authentication/session behavior, WebView2 behavior,
restart/persistence, secure mode, uninstall, and data preservation remain pending.
The existing bounded native-route evidence is retained at
`reports/release-readiness/r4-native-windows-recovery.md`; no Wine result is used
as acceptance evidence.

R4 was approved by ASTRA_HIGH for the closest supported cross-built state, and R5
was approved as scoped cross-platform acceptance. Neither approval claims Windows
native runtime readiness.

## Final authority

- **Decision:** `APPROVE_PLATFORM_SCOPED_RELEASE`
- **Authority:** ASTRA_HIGH via `openai-codex / gpt-6-astra`
- **Authority artifact:** `reports/release-readiness/final-astra-high-platform-scoped-approved.json`
- **Authority session:** `20260908_093759_6741e1`
- **Final manifest status:** Linux is `READY / NATIVE VALIDATED`; Windows is `INSTALLER AVAILABLE / CROSS-BUILT ON LINUX / NATIVE WINDOWS RUNTIME VALIDATION PENDING`.
- **Native Windows runtime claim:** `false`

## Governance and spend

- No new feature, research, redesign, optimization, or speculative card was created.
- No duplicate Windows recovery card was created.
- The previously completed R4 recovery card remains the sole recovery action.
- The final Linux build used the local authorized toolchain and incurred no incremental spend.
- No commit was pushed.
