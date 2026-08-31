# Program Design: Historical application entry

## Files
- `crates/domain/src/lib.rs` — add backward-compatible application provenance/date fields to `IpoApplicationCreated`.
- `crates/local-index/src/lib.rs` — schema v7 columns and projection of the new application fields.
- `crates/local-index/tests/allotment_runtime.rs` — prove schema v7 contains no PAN-bearing columns and preserves defaults.
- `apps/desktop/src-tauri/src/service.rs` — narrow request/response DTOs, validation, and event-chain orchestration.
- `apps/desktop/src-tauri/src/lib.rs` — expose one typed Tauri command.
- `apps/desktop/src-tauri/tests/service_flow.rs` — focused end-to-end service/projection/security check.
- `apps/desktop/src/App.tsx` — distinct historical-entry mode within Invest; reuse existing controls/styles.
- `apps/desktop/src/App.flows.test.tsx` — prove owner affirmation and safe command payload.
- `docs/plans/historical-application-entry/*` — approved workflow state and decisions.

## Types & signatures

```rust
// sanket-domain
EventPayload::IpoApplicationCreated {
    application_id: String,
    session_id: String,
    ipo_name: String,
    planned_amount_paise: i64,
    registrar_id: String,
    registrar_name: String,
    official_status_url: Option<String>,
    expected_allotment_date: Option<String>,
    source: String,                  // serde default: OWNER_CURRENT_ENTRY
    application_date: Option<String>, // YYYY-MM-DD when known
}
```

```rust
#[derive(Debug, Deserialize)]
pub struct HistoricalApplicationRequest {
    pub actor_member_id: String,
    pub account_id: String,
    pub ipo_name: String,
    pub amount_paise: i64,
    pub application_date: Option<String>,
    pub registrar_id: String,
    pub provider_issue_id: String,
    pub owner_affirmed: bool,
}

#[derive(Debug, Serialize)]
pub struct HistoricalApplicationResponse {
    pub session_id: String,
    pub application_id: String,
    pub allocation_id: String,
    pub provider_id: String,
    pub provider_issue_id: String,
    pub source: String,
}

impl Application {
    pub fn record_historical_application(
        &self,
        req: HistoricalApplicationRequest,
    ) -> Result<HistoricalApplicationResponse>;
}

#[tauri::command]
fn record_historical_application(
    state: tauri::State<'_, AppState>,
    request: HistoricalApplicationRequest,
) -> Result<HistoricalApplicationResponse, String>;
```

```ts
interface HistoricalApplicationDraft {
  ipoName: string;
  amountRupees: string;
  applicationDate: string;
  registrarId: "mufg_intime";
  providerIssueId: string;
  accountId: string;
  ownerAffirmed: boolean;
}
```

## Call stack

### Save historical application
1. `HistoricalApplicationForm.submit`
2. `bridge.invoke("record_historical_application", { request })`
3. Tauri `record_historical_application`
4. `Application::record_historical_application`
5. Validate owner affirmation, amount, optional date, owner account, registrar, issue id, and PAN-free public fields
6. `ProviderRegistry::resolve_registrar`
7. Build sealed events with current `occurred_at`:
   - `InvestmentSessionCreated`
   - `IpoApplicationCreated` (`OWNER_HISTORICAL_ENTRY`, optional application date)
   - `AllocationAdded`
   - `AllotmentProviderDiscovered`
   - `InvestmentSessionSubmitted`
8. Existing `MemberVault::append_event` + `LocalIndex::apply_event`
9. Return opaque ids and refresh dashboard/allotment candidates

### Current investment submit
Existing call stack remains unchanged; it writes `OWNER_CURRENT_ENTRY` and no application date.

## Test plan
- `historical_application_requires_owner_affirmation` — fails closed before writes.
- `historical_application_records_owner_chain_mapping_without_job_or_pan` — one owner, ₹14,820, unknown date produces session/application/allocation/mapping, source/date semantics, no job/result, no PAN in SQLite/events.
- `historical_application_rejects_non_owner_account` — friend/unknown account cannot masquerade as the owner historical application.
- `schema_v7_adds_application_provenance_without_sensitive_columns` — migration and defaults are safe.
- `owner_can_submit_historical_application_without_recommendation_or_lookup` — frontend sends only safe fields, requires affirmation, and displays safe opaque application id.

## Least confident decisions
1. `OWNER_CURRENT_ENTRY` is the backward-compatible default name; it is explicit but existing entries were not necessarily owner-entered. Alternative: `LEGACY_UNSPECIFIED` for replayed old events and `OWNER_CURRENT_ENTRY` only for new normal submits.
2. The historical form is a mode inside Invest rather than a new navigation item; this minimizes UI surface while keeping the flow owner-controlled.
3. One historical application amount also serves as the one-application session's declared capital; this preserves the current accounting invariant without inventing a second amount.
