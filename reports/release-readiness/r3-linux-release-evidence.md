# SANKET-R3 — Linux Release Readiness Evidence

- **Recorded:** 2026-09-07T14:51:28+05:30
- **Card:** `t_ea77e578`
- **Scope:** existing Linux release packaging and secure-default repair only; no feature expansion, provider integration, financial mutation, commit, or push.

## Bounded security repair

ASTRA_HIGH returned `REVISE` before implementation because a release-default change alone would still permit an explicit `DEVELOPMENT_SYNTHETIC` environment value. The smallest approved-by-scope remediation was applied:

- `service::resolve_security_mode_for_build` centralizes mode resolution.
- Release builds default to `PRODUCTION_SECURE` when the environment is absent.
- Release builds coerce explicit/unknown synthetic values back to `PRODUCTION_SECURE`.
- Debug builds retain explicit `DEVELOPMENT_SYNTHETIC` for development/test use.
- Tauri setup and `AppState::status()` use the shared resolver/label.
- No authentication architecture, key provider, PAN, provider, or storage boundary was redesigned.

ASTRA_HIGH remediation-strategy session: `20260907_141259_570ade`.

## Tests and build

- RED observed before production implementation: `cargo test -p sanket-desktop --test key_provider_gate` failed because `resolve_security_mode_for_build` did not exist.
- GREEN: `cargo test -p sanket-desktop --test key_provider_gate` PASS, 5 tests.
- `cargo test --release -p sanket-desktop --test key_provider_gate` PASS, 5 tests.
- `cargo test -p sanket-desktop --lib native_auth_command_tests` PASS, 6 tests.
- `cargo fmt --all -- --check` PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` PASS.
- `cargo test --workspace` PASS.
- `npm run check` PASS: 221-file secret scan, format, typecheck, frontend tests, Python tests.
- `git diff --check` PASS.
- `apps/desktop/src-tauri/tauri.conf.json` was minimally formatter-normalized before the clean rebuild; no semantic bundle configuration changed.

## Linux artifact

Command: `npm run tauri:build -- --bundles deb --ci --no-sign`

- Build invocation count: `1` from the exact source snapshot below; no rebuild occurred after hashing.
- Unbundled Tauri target: `target/release/sanket-ipo`
- Unbundled target SHA-256: `f366d8fdc052c9e1d1cad6b76cfcbe6c8e7e90520c8976b910331334b32d15f7`; size 20,236,048 bytes; mode `755`; ownership `1000:1000`; bundle marker `UNK`.
- Final packaged release binary: extracted DEB payload `/usr/bin/sanket-ipo`
- Packaged payload SHA-256: `a6d80bd8b954ef4e9fca1b7e826592b6552fcf08fd5419d06bb10b09f9aeb4b0`; size 20,236,048 bytes; mode `755`; ownership `1000:1000`; bundle marker `DEB`.
- Package: `target/release/bundle/deb/Sanket IPO_0.1.0_amd64.deb`
- Package SHA-256: `3cb2130d55db9796f55a4a0c75865af333de13dfb1ce8f9401b3e733ca405b47`
- Package size: 6,551,632 bytes.
- The standalone target and packaged payload have the same ELF Build ID and size. Their byte hashes differ because Tauri rewrites the bundle marker from `UNK` to `DEB`; the DEB payload is the final packaged release binary and is authoritative.
- Package metadata: Debian `amd64`, version `0.1.0`, dependencies `libwebkit2gtk-4.1-0`, `libgtk-3-0`.
- Package payload: executable, desktop entry, and `/usr/share/icons/hicolor/512x512/apps/sanket-ipo.png`.
- Desktop entry: `Exec=sanket-ipo`, `Icon=sanket-ipo`, `Terminal=false`, `Type=Application`.
- No maintainer scripts were present; control archive contained only `control` and `md5sums`.
- Package payload paths are canonical and contain no bundled secrets.

RPM and AppImage were not produced by the exact one-bundle command; DEB is the supported Linux package for this run.

## Exact source and build provenance

- Provenance artifact: `reports/release-readiness/r3-linux-release-provenance.json`
- Source snapshot identifier: `f6554fc7c7228e8f27a26f16a8a75956732acfdf64275ca0406b6daf07c3f87`
- Git HEAD: `d3864557ee56282214733d44149cb2ed1fbc0ee6`
- Git branch: `feature/aether-ui-redesign`
- Pre-build `git status --porcelain` SHA-256: `ecd864b02569803847f1c675241b405eb73e04eefa493ef39575d86ffc17625c`
- Pre-build `git diff` SHA-256: `c1bb3201e918234717b06cb0a3193c8245d8114d0eb0c9bb640a40b793898cd5`
- Pre-build `git diff --cached` SHA-256: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- `package-lock.json` SHA-256: `1e555f631520fd462d4f17a79a97306547502d071c5259a4998f47190bc3b87d`
- `Cargo.lock` SHA-256: `fccd660464a349ba0d8379060f86db7d7791b29faaefd89b56492cbc4ce37422`
- Security mode: `SANKET_SECURITY_MODE` unset; release resolver selects `PRODUCTION_SECURE` and rejects synthetic mode.
- Exact build command: `npm run tauri:build -- --bundles deb --ci --no-sign`
- Build flags/toolchains are fully recorded in `r3-linux-release-provenance.json`; post-build source comparison matched all seven source identifiers.

## Install/uninstall and clean-profile checks

- `dpkg-deb --extract` and `--control` passed in disposable `/tmp` roots.
- Unprivileged native `dpkg --unpack/--remove` was attempted only against a disposable root and correctly refused with `requested operation requires superuser privilege`; no host package state changed.
- `fakeroot dpkg --unpack` passed in `/tmp/sanket-r3-dpkg-root`; package query returned `install ok unpacked`; `fakeroot dpkg --remove` passed and the executable was removed. A second disposable install/remove with a pre-existing user-data marker returned `unpack=0 remove=0 user_data_preserved=0`; the marker remained.
- Release binary launched with `SANKET_SECURITY_MODE` unset, isolated `HOME`/XDG directories, `DISPLAY=:1`, and `dbus-run-session`; it initialized `settings.json`, `index.sqlite3`, and the member-vault directory before the planned 12-second termination (`exit 124`). This proves bounded clean-profile startup/initialization, not an interactive GUI walkthrough.
- Clean-profile app data was owned by the normal user (`uid=1000 gid=1000`); `settings.json` and `index.sqlite3` were mode `600`.
- Two isolated startup runs preserved `settings.json` and the SQLite projection across restart; schema inspection found `masked_pan` and no full/raw PAN column.
- The initialized projection schema contains `masked_pan` only; no full PAN column.
- No PAN, credential value, keyring value, live provider/MUFG request, financial mutation, commit, or push occurred.

## Remaining release gates

This evidence supports Linux artifact/package construction and secure release-default behavior. It does not claim Windows build/installer/Credential Manager evidence, full interactive Linux acceptance, cross-platform acceptance, R5, or SANKET-FINAL release approval.

Protected HEAD remains `d3864557ee56282214733d44149cb2ed1fbc0ee6`; the worktree remains intentionally dirty.
