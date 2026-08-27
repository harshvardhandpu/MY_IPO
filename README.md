# Sanket IPO

Private, local-first Windows/Linux desktop operating system for a trusted Indian IPO investment group.

## Trust boundary

- **Member domain:** private identities, allocations, proofs, accounting, audit. External AI denied.
- **Intelligence domain:** public IPO filings, news, algorithms, research, backtests. External AI allowed through an allowlisted gateway.
- **Allotment worker:** deterministic local automation with temporary purpose-scoped identity access. It never calls an LLM.

The durable shared truth is append-only Git/Obsidian-compatible records. SQLite is a local reconstructable projection and is never synchronized as the shared database.

## Stack

Tauri 2 · React · TypeScript · Vite · Rust · SQLite · private Git synchronization.

## Development

Prerequisites: Node.js 22.22.2+, npm, Rust 1.85+, and the platform dependencies listed by Tauri 2.

```bash
npm install
npm run check
npm run tauri:dev
```

Synthetic data only during development. Never commit PAN, UPI IDs, API keys, proof bytes, vault contents, or local databases.

## Source of truth

`docs/planning/SANKET_IPO_MASTER_SOURCE.md`
