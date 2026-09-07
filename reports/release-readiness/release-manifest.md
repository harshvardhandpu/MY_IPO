# Sanket IPO v1 Release Manifest

Generated: 2026-09-07T18:10:56+05:30

## Release status

**SANKET IPO v1**

**LINUX:** READY / APPROVED

**WINDOWS:** PENDING NATIVE VALIDATION

**SECURITY:** PASS

**AUTH:** PASS

**DATA INTEGRITY:** PASS

**ZERO-INCREMENTAL-SPEND:** PASS

**LINUX_V1_READY = YES**

The Linux release is ready for use. Windows is a separate platform-readiness
blocker and is not represented as ready by this manifest.

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

## Windows status

**KNOWN BLOCKER:** Native Windows execution environment unavailable.

R4 remains blocked only by the previously verified missing native Windows
execution capability. The existing bounded recovery evidence is retained at
`reports/release-readiness/r4-native-windows-recovery.md`. No Wine result,
cross-compile result, or static inspection is treated as native Windows
acceptance evidence.

Required Windows evidence remains outstanding, including the native Windows
build, installer and executable hashes, Credential Manager behavior,
Windows app-data paths, clean-profile startup, restart/persistence, secure mode,
and uninstall/data-preservation checks.

## Governance and spend

- No new feature, research, redesign, optimization, or speculative card was created.
- No duplicate Windows recovery card was created.
- The previously completed R4 recovery card remains the sole recovery action.
- The final Linux build used the local authorized toolchain and incurred no incremental spend.
- No commit was pushed.
