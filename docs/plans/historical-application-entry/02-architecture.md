# Architecture: Historical application entry

## Fit
- Reuse the existing Invest screen and add a distinct owner-controlled historical-entry form; the current recommendation/new-investment path remains unchanged.
- Add one narrow Tauri command backed by the existing desktop `Application` service.
- Reuse the existing event/vault/projection pipeline to create the session, application, allocation, submitted-session, and public provider-mapping records atomically in intent order.
- Reuse the existing owner member/account and encrypted identity; the command carries only the opaque account id and never accesses PAN.
- Reuse `ProviderRegistry` for MUFG registrar identity and official status URL.

## Endpoints
- `record_historical_application` (Tauri command) — validate and persist one owner-affirmed historical application; performs no provider request.

## Data
- Add `source` and nullable `application_date` to the application projection. Existing records default to the current-entry source; historical entries use `OWNER_HISTORICAL_ENTRY`.
- Extend the application-created event with backward-compatible defaulted provenance/date fields.
- Persist `AllotmentProviderDiscovered` for application/provider `mufg-intime-live`, public issue `11926`, and the registry-owned official URL.
- No allotment job or attempt is created; automated result remains absent.

Queries:
- Validate the supplied account id belongs to an existing owner member.
- Project one submitted session, one application, one allocation, and one provider issue mapping.
- Read the resulting application/allocation/mapping for verification.

## Flow
1. Owner opens Add historical application in the native Invest view.
2. UI collects IPO, amount, optional application date, registrar, account, public issue id, and explicit affirmation.
3. Tauri validates the narrow request and calls `Application::record_historical_application`.
4. Service validates positive amount, optional ISO date, owner account, MUFG registrar, issue id, and no embedded PAN.
5. Service emits and projects session-created → application-created (`OWNER_HISTORICAL_ENTRY`) → allocation-added → provider-discovered → session-submitted events.
6. UI refreshes dashboard/candidates and reports safe opaque application id.
7. No lookup authorization is changed; no HTTP request is issued.

## External
None during save. MUFG is not contacted. The provider issue id is owner-confirmed public data, and the official status URL comes from the existing registrar registry.
