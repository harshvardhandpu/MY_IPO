# SANKET-R4 — Windows Release Readiness Evidence

- Recorded: 2026-09-07
- Card: `t_5ca8e730`
- Scope: Windows portability/build/package evidence only; no feature expansion, provider request, PAN, credential value, keyring value, financial mutation, commit, or push.
- Result: `PARTIAL — BLOCKED_ON_WINDOWS_RUNTIME_HOST`

## Build and compile evidence

The repository was cross-checked and release-built for `x86_64-pc-windows-gnu` in an isolated Arch Linux container with MinGW-w64. The build consumed the current dirty worktree at protected HEAD `d3864557ee56282214733d44149cb2ed1fbc0ee6`; `target-win` was moved outside the repository after build so generated Cargo JSON could not contaminate repository checks.

- `cargo check --workspace --target x86_64-pc-windows-gnu`: PASS.
- `cargo build --release -p sanket-desktop --target x86_64-pc-windows-gnu`: PASS.
- `npm run build`: PASS.
- Windows release executable: `/home/harshdev/.cache/sanket-r4-artifacts-target-win/x86_64-pc-windows-gnu/release/sanket-ipo.exe`
- Executable size: 31,274,471 bytes.
- Executable SHA-256: `0d04dc00726423694a4036450d6acb91d56b44c7d38c234140896e4db2d40311`.
- File identification: PE32+ GUI executable for MS Windows, x86-64.

## Installer evidence

Tauri generated the NSIS script and Windows bundle inputs from the same release output. The Linux host lacked `makensis.exe`, so the generated script was executed with Debian NSIS 3.08 in a disposable container, using the Tauri `nsis_tauri_utils.dll` plugin. This produced an unsigned NSIS setup executable; this is a cross-build artifact, not Windows-host installer acceptance.

- Installer: `/home/harshdev/.cache/sanket-r4-artifacts-target-win/x86_64-pc-windows-gnu/release/bundle/nsis/Sanket IPO_0.1.0_x64-setup.exe`
- Installer size: 6,651,857 bytes.
- Installer SHA-256: `9a273ec5f7898ea765369c4dbf5a550a3a7bdf8599772eece22758b5b0bd1249`.
- Installer type: PE32 Nullsoft Installer self-extracting archive.
- Signing: not performed; no signing key was accessed.
- MSI: not produced. Tauri documents WiX/MSI as Windows-host-only; NSIS is the supported cross-build path used here.

## Platform portability inspection

- Runtime data paths use Tauri `app.path().app_data_dir()` for `settings.json`, `index.sqlite3`, `member-vault`, and public cache; no hard-coded Unix home or XDG path is used by application startup.
- OS credential provider is the existing `keyring` `windows-native` backend. Windows-specific keyring code compiled as part of the Windows release target.
- Unix permission code is guarded by `cfg(unix)`; the non-Unix path compiles without Unix filesystem APIs.
- Tauri bundle configuration has bundling enabled with the application icon. The Tauri schema default for NSIS `installMode` is `currentUser`, which does not require administrator access; installer runtime verification remains outstanding.

## Unverified acceptance criteria

The current host is Linux. No real Windows runner/host was available for the required runtime evidence:

- Windows Credential Manager create/read/restart/failure behavior: NOT VERIFIED.
- Windows first launch and authentication smoke: NOT VERIFIED.
- Windows app-data path and ACL readback: NOT VERIFIED.
- Windows restart/persistence: NOT VERIFIED.
- Clean-profile NSIS install without elevation: NOT VERIFIED.
- Clean-profile uninstall and user-data preservation: NOT VERIFIED.
- Windows WebView2 runtime behavior: NOT VERIFIED.

Wine was attempted with an isolated prefix. `wineboot` initialized, but the NSIS installer could not complete a reliable unattended install and the GUI run timed out; this is not accepted as Windows evidence.

## Other gates

- `cargo test --workspace`: PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `npm run check`: BLOCKED by a pre-existing Biome formatting diagnostic in `reports/release-readiness/r3-linux-release-provenance.json`; no source file was changed to alter the already-approved R3 evidence.
- `git diff --check`: PASS.
- Secret scan: PASS when run as part of `npm run check` before its format stage; no secret or sensitive value was accessed.

R4 cannot be closed or submitted for Windows phase approval until a Windows runner/host produces the required Credential Manager, clean-profile install/uninstall, first-launch, auth, restart, persistence, and no-admin evidence. No commit or push was performed.
