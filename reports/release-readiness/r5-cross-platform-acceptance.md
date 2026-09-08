# SANKET-R5 — Cross-Platform Acceptance Evidence

- **Recorded:** 2026-09-08
- **Card:** `t_81d64947` (`SANKET-R5`)
- **Authority scope:** Cross-platform release acceptance under the user-authorized Windows cross-build policy.

## Required platform statuses

- `LINUX = NATIVE VALIDATED / APPROVED`
- `WINDOWS = CROSS-BUILT ARTIFACT AVAILABLE`
- `WINDOWS_NATIVE_RUNTIME_VALIDATION = PENDING`
- `CROSS_PLATFORM_NATIVE_RUNTIME_VALIDATED = false`

The Windows limitation is explicit and does not revoke Linux readiness.

## Accepted source binding

- Current repository HEAD: `e2a17b8d2115b200bf0262ff03fd0b44364e1518`
- Current source archive SHA-256: `f39787381b965dc01673ce2cf1d272a7ee6bd582507dc1bb7d6186f96b4f0321`
- Accepted product source commit: `6afe4d75dd6fb0840f82714cecd5c707cfe5f571`
- `git diff 6afe4d75..HEAD` contains only the release checkpoint and release manifest; no product source changed after the accepted Linux freeze.
- `package-lock.json` SHA-256: `1e555f631520fd462d4f17a79a97306547502d071c5259a4998f47190bc3b87d`
- `Cargo.lock` SHA-256: `fccd660464a349ba0d8379060f86db7d7791b29faaefd89b56492cbc4ce37422`
- Worktree: clean
- `git diff --check`: PASS

## Linux — native approved

The existing R3 ASTRA_HIGH approval remains closed and was not reopened.

- Package: `target/release/bundle/deb/Sanket IPO_0.1.0_amd64.deb`
- Version: `0.1.0`
- Architecture: `amd64`
- DEB SHA-256: `0d823dce7cb003a003792673e93cf0e1b607794ee40e4890e6593c8df5035e08`
- DEB size: `6,551,626` bytes
- Extracted payload: `/usr/bin/sanket-ipo`
- Extracted payload SHA-256: `a6d80bd8b954ef4e9fca1b7e826592b6552fcf08fd5419d06bb10b09f9aeb4b0`
- Extracted payload size: `20,236,048` bytes
- Linux install/start/restart/persistence/uninstall/data-preservation evidence: PASS in the approved R3 release manifest.
- R3 authority artifact: `reports/release-readiness/r3-astra-high-approved.json`

## Windows — cross-built artifact available

- Target: `x86_64-pc-windows-msvc`
- Application: `target/x86_64-pc-windows-msvc/release/sanket-ipo.exe`
- Application SHA-256: `1c11bc1d1fbbdefbbab184cc3a3376748d63c218ee5f94b7c4e44e818f9c9882`
- Application size: `15,137,792` bytes
- NSIS installer: `target/x86_64-pc-windows-msvc/release/bundle/nsis/Sanket IPO_0.1.0_x64-setup.exe`
- NSIS installer SHA-256: `5ec68f092e6fc0606df17cfd387f5e258896be13c79eda195020875c0ea886dc`
- NSIS installer size: `4,106,322` bytes
- Installer payload extracted successfully with 7-Zip.
- Extracted `sanket-ipo.exe` SHA-256 exactly matches the standalone application.
- Standalone and extracted application are PE32+ AMD64 (`0x8664`, optional header `0x20B`).
- The NSIS bootstrap stub is PE32 i386 from the Debian NSIS toolchain; this does not change the verified AMD64 application payload.
- Windows artifact evidence: `reports/release-readiness/r4-windows-cross-build-evidence.md`
- R4 ASTRA_HIGH approval: `reports/release-readiness/r4-astra-high-cross-build-approved.json`

## Security and integrity

- `cargo xwin check --workspace --target x86_64-pc-windows-msvc`: PASS.
- `npm run secrets`: PASS (`225` files checked).
- Binary/PAN/credential-marker scans: PASS; no format-valid PAN, private-key marker, common live-token format, JWT-like value, or API-key/secret assignment literal found in the standalone installer/payload scans.
- Production frontend has no Vite dev-server or localhost markers.
- No provider, PAN, credential value, keyring value, financial mutation, paid CI, paid signing, or Wine evidence was used.

## Explicit limitations

The following are **not** claimed for Windows:

- native Windows installer execution;
- Windows Credential Manager behavior;
- Windows app-data paths/ACLs;
- first launch or authenticated login/session behavior;
- WebView2 behavior;
- restart/persistence;
- secure-mode runtime behavior;
- uninstall/data preservation;
- native Windows runtime smoke testing of any kind.

`WINDOWS_NATIVE_RUNTIME_VALIDATED = false` remains authoritative.

## R5 authority state

- Cross-platform evidence package: **READY FOR ASTRA_HIGH**.
- Requested decision: approve the scoped cross-built artifact state and activate SANKET-FINAL while preserving the native Windows limitation.
- R5 decision: **APPROVE_CROSS_PLATFORM_SCOPED** by ASTRA_HIGH.
- Authority artifact: `reports/release-readiness/r5-astra-high-cross-platform-approved.json`.
- Native runtime limitation remains authoritative: `WINDOWS_NATIVE_RUNTIME_VALIDATED = false`.
