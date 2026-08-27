# Architecture

## System shape

Sanket IPO is one Tauri desktop product with modular Rust services and a React UI. It is not a networked microservice system.

```text
React UI
  │ typed Tauri commands (no arbitrary fs/shell)
  ▼
Application services
  ├── Private domain ── MemberVault events ── SQLite projection
  ├── Local allotment worker ── purpose-scoped PAN capability ── registrar adapter
  └── Public intelligence ── IntelligenceVault ── AI gateway
                                      ▲
                                  no private edge
```

## Durable and derived state

- **Durable truth:** immutable, schema-versioned records in separate Git-backed vaults.
- **Derived state:** local SQLite materialized projection. Delete/rebuild must be supported.
- **Proofs:** compressed, hashed, encrypted, content-addressed blobs.
- **Secrets:** OS credential store; encrypted enrollment bundle may be synchronized, plaintext may not.

## Workspace boundaries

- `apps/desktop`: React UI and least-privilege Tauri command adapter.
- `crates/domain`: dependency-light domain types, roles, sync states, and hashed event envelope.
- `crates/device-settings`: atomic local settings and stable device identity.
- `crates/local-index`: restricted-permission SQLite projection and embedded migrations.
- `crates/member-vault`: private event/blob persistence capability only.
- `crates/intelligence-vault`: public intelligence persistence capability only.
- `crates/sync-engine`: debounced Git synchronization contracts and state machine.
- `crates/crypto`: envelope encryption and redaction primitives.
- `crates/audit`: immutable security/financial audit events.
- `crates/calculation-engine`: allocation-level monetary calculations.
- `crates/ai-gateway`: only public/sanitized request types.
- `crates/strategy-engine`: public-data-only research/backtesting.

## Command boundary

The webview receives narrow commands such as `get_app_status` and `submit_investment`; it never receives general filesystem or shell capabilities. Rust validates all inputs before state changes.

## Failure model

Local event persistence is the success boundary for user writes. Projection, Git sync, AI, news, and registrar work happen asynchronously and expose explicit status. Unknown registrar output remains `UNKNOWN`, never `NOT_ALLOTTED`.

## Significant decisions

1. npm workspaces and a Cargo workspace keep one repository without introducing a monorepo service framework.
2. System Git may be wrapped initially; the interface permits a later libgit2 implementation.
3. SQL migrations are embedded and local-only; operational event schemas remain portable files.
