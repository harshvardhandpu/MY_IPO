# Sync Protocol

## Principle

Local save is immediate. Git synchronization is asynchronous and must not block accounting.

## Write path

1. Validate command.
2. Atomically write immutable local event/blob.
3. Apply event to SQLite projection transactionally.
4. Return success to UI as `Pending`.
5. Queue debounced sync work.

## Schedule

- Push debounce: configurable 3–10 seconds after last write.
- Active fetch: configurable 15–30 seconds.
- Fetch on startup, focus, and reconnect.
- Manual `Sync Now` is always available.

## States

`Synced`, `Syncing`, `Pending`, `Offline`, `Conflict`, `AuthenticationRequired`.

## Pull/merge

1. Fetch remote.
2. Merge append-only paths.
3. If same event path differs, stop and mark integrity conflict; never choose silently.
4. Validate schema/hash for new events.
5. Quarantine invalid events.
6. Project only accepted new events.
7. Rebase/retry local append-only commits conservatively, then push.

## Recovery

- Local writes remain queued offline.
- SQLite can be deleted and rebuilt from accepted vault events.
- A failed push never rolls back a committed local event.
- Persist sync queue/last accepted event cursor locally.
- Exponential backoff with jitter; authentication errors require owner/member action and are not retried aggressively.

## Git policy

No per-keystroke commits and no synchronized SQLite. Batches use sanitized semantic messages containing opaque IDs only.
