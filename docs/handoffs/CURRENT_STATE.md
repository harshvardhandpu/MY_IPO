# Current State

- **Branch:** `feature/foundation`
- **Latest commit:** none yet (unborn repository)
- **Phase:** Phase 0 repository baseline in progress

## Completed

- Verified the supplied repository and correct GitHub remote.
- Read the complete v2 master source and copied it to the canonical path.
- Verified host Node/npm, Rust/Cargo, GitHub CLI authentication, and Linux Tauri libraries.

## Tests

- Baseline project had no code or tests.

## Known failures / blockers

- Dual-graph project tools were unavailable in the active Hermes toolset; repository inspection used the two known files only.
- The repository starts with uncommitted source files and no commits; work is isolated on `feature/foundation`.

## Architecture decisions

- Tauri 2 + React/TypeScript + Rust workspace.
- Append-only Git/Obsidian events are durable truth; SQLite is a local rebuildable projection.
- Member, sensitive allotment automation, and public/AI domains use structural dependency boundaries.

## Next exact tasks

1. Finish Phase 0 safeguards/docs and run secret scanning.
2. Scaffold the runnable Tauri shell and Rust workspace.
3. Add tested event, device/settings, audit, and SQLite foundation.

## Commands

```bash
npm install
npm run check
npm run tauri dev
```
