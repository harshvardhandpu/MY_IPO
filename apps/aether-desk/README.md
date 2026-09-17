# Aether IPO working desk

Isolated frontend desk for review. It is **not** the Tauri production shell and
it does **not** talk to KFintech, MUFG Intime, or Bigshare.

## Runtime mode

This package is `DEVELOPMENT_SYNTHETIC` only:

- identity is last-four characters, stored locally
- no PAN is accepted, persisted, or transmitted
- allotment rows are simulated fail-closed outcomes for UI review
- `NOT_FOUND`, captcha, and Bigshare human verification stay unresolved
- official registrar pages remain the source of truth

Production allotment transport, MemberVault, lookup authorization, and SQLite
event authority live in the Rust crates and are untouched by this package.

## Run

From the repo root:

```bash
npm install
npm run dev --workspace @sanket/aether-desk
```

The desk is a full-bleed dark viewport: Dashboard, Members, Invest
(CHECK vs SUBMIT), Check Allotment, Investments / historical, Books, and Calendar.
