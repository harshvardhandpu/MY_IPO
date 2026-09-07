# SANKET-R4-RECOVERY — Native Windows Execution Route Audit

- Recorded: 2026-09-07T17:34:41+05:30
- Recovery card: `t_91d31dff`
- Parent gate: `SANKET-R4` (`t_5ca8e730`)
- Blocker classification: **A — missing native Windows execution environment**
- Recovery result: `NO_VALID_ZERO_SPEND_NATIVE_ROUTE_AVAILABLE`
- R3 status: preserved; no R3 evidence was reopened or changed.

## Authoritative source state

- Local HEAD: `d3864557ee56282214733d44149cb2ed1fbc0ee6`
- Remote `feature/aether-ui-redesign` HEAD: `24258baaa65a5af011d29717e6d9002b38c24527`
- The current worktree is dirty and contains source changes not present on the remote branch.
- No commit, tag, or push was created by this recovery attempt.
- Existing partial R4 evidence remains bound to local protected HEAD `d3864557ee56282214733d44149cb2ed1fbc0ee6`; it is not upgraded by this audit.

## Route 1 — existing native Windows host/runner

- Current host: Linux `x86_64`.
- `powershell.exe`, `pwsh`, `wsl.exe`, and native Windows execution tools are absent.
- No local GitHub Actions runner installation or runner process was found.
- No configured Windows host alias was found; the only SSH alias is unrelated and was not contacted.
- Result: **UNAVAILABLE**.

## Route 2 — existing GitHub Actions `windows-latest`

The repository has an existing `.github/workflows/ci.yml` with `windows-latest` jobs for frontend and Rust checks.

Verified constraints:

- Repository visibility is `private`.
- Actions are enabled; the workflow is active.
- No self-hosted runner is registered (`total_count=0`).
- The workflow uses standard `windows-latest`, not a larger runner.
- Repository workflow permissions are read-only.
- Repository-level Actions secrets and variables lists are empty; no project secret was read or exposed.
- The workflow has no `workflow_dispatch` trigger; it runs on push or pull request.
- The remote branch is at `24258baaa65a5af011d29717e6d9002b38c24527`, while the accepted local source is dirty at `d3864557ee56282214733d44149cb2ed1fbc0ee6` plus uncommitted changes. A remote run would not test the current accepted source state.
- GitHub Actions billing data could not be read with the current token scope; the API explicitly requires the `user` scope. No interactive authentication, billing upgrade, or token-scope change was attempted.
- Because the repository is private and the billing allowance/overage protection is not verifiable, this route cannot be classified as `ZERO_INCREMENTAL_SPEND`.
- Triggering a run for the current dirty source would require a source commit/push or equivalent remote source publication, which was not authorized in this recovery attempt.

Authoritative GitHub documentation consulted:

- `https://docs.github.com/en/actions/reference/runners/github-hosted-runners`
- `https://docs.github.com/en/billing/concepts/product-billing/github-actions`

Those sources state that standard hosted runners are free and unlimited for public repositories, while private repositories consume plan minutes and can be billed beyond included minutes. This repository is private, so the public free rule does not apply.

Result: **DISQUALIFIED — zero spend and current-source provenance cannot both be proven**.

## Route 3 — connected CI capable of native Windows execution

- No connected CI service or authorized Windows runner beyond the GitHub Actions workflow was found in the repository or native runner inventory.
- No paid fallback or external service was contacted.
- Result: **UNAVAILABLE**.

## Route 4 — existing authorized Windows machine/runner

- No configured authorized Windows machine or registered self-hosted runner was found.
- Result: **UNAVAILABLE**.

## Recovery decision

The bounded recovery attempt completed without a valid route. This is not a human-only blocker and is not an Astra-wait state. It is a typed capability block: the project currently lacks an auditable, zero-incremental-spend native Windows execution route for the dirty accepted source state.

Preserve:

- `SANKET-R3 = DONE`.
- `SANKET-R4 = BLOCKED(capability)`.
- `SANKET-R5 = TODO`.
- `SANKET-FINAL = TODO`.

Do not invoke ASTRA_HIGH for R4 from partial cross-target evidence. R4 authority requires the native Windows evidence listed on the R4 card.

No PAN, credential value, keyring value, provider request, financial mutation, secret access, commit, or push occurred.
