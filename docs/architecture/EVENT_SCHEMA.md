# Event Schema

## Envelope v1

```json
{
  "schema_version": 1,
  "event_id": "opaque",
  "event_type": "MEMBER_CREATED",
  "aggregate_type": "CORE_MEMBER",
  "aggregate_id": "opaque",
  "aggregate_revision": 1,
  "actor_member_id": "opaque",
  "device_id": "opaque",
  "occurred_at": "2026-08-28T00:00:00Z",
  "app_version": "0.1.0",
  "payload": {},
  "previous_event_hash": null,
  "content_hash": "sha256:..."
}
```

The canonical hash covers the envelope excluding `content_hash`, using deterministic JSON serialization. Invalid hashes/schemas are quarantined and never projected.

## Storage path

`events/<member-id>/<device-id>/<YYYY>/<MM>/<event-id>.json`

No filename contains PAN, UPI, email, phone, or display name.

## Initial event types

`MEMBER_CREATED`, `FRIEND_ADDED`, `FRIEND_ARCHIVED`, `INVESTMENT_SESSION_CREATED`, `IPO_APPLICATION_CREATED`, `ALLOCATION_ADDED`, `PROOF_ATTACHED`, `ALLOTMENT_CHECK_STARTED`, `ALLOTMENT_RECORDED`, `PROFIT_RECORDED`, `FRIEND_SHARE_DUE`, `FRIEND_SHARE_PAID`.

## Payload constraints

- Identity events store `sensitive_record_id`, `masked_pan`, and consent metadata only.
- Proof events store encrypted blob references/hashes only.
- Allotment results store account ID and normalized safe provider metadata, never PAN.
- Monetary fields use integer paise; share rates use basis points.

## Merge/projection rules

- Event ID is globally unique; duplicate identical events are idempotent.
- Same ID with different content is an integrity conflict.
- Aggregate revisions must be contiguous; gaps remain pending until predecessors arrive.
- Archival is a state transition, never physical financial-history deletion.
