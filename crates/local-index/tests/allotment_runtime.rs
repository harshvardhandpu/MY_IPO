use sanket_domain::{EventEnvelope, EventPayload, NewEvent};
use sanket_local_index::LocalIndex;

fn seal(event_id: &str, payload: EventPayload) -> EventEnvelope {
    EventEnvelope::seal(NewEvent {
        event_id: event_id.into(),
        aggregate_type: "allotment".into(),
        aggregate_id: "job-1".into(),
        aggregate_revision: 1,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-08-28T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload,
    })
    .unwrap()
}

fn created() -> EventEnvelope {
    seal(
        "event-created",
        EventPayload::AllotmentJobCreated {
            job_id: "job-1".into(),
            application_id: "app-1".into(),
            session_id: "session-1".into(),
            ipo_name: "Example IPO".into(),
            registrar_id: "kfintech".into(),
            registrar_name: "KFintech".into(),
            official_status_url: Some("https://ipostatus.kfintech.com".into()),
            provider_id: "kfintech-fixture".into(),
        },
    )
}

#[test]
fn schema_v8_adds_durable_runtime_tables_without_pan_columns() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    assert_eq!(index.schema_version().unwrap(), 8);
    for table in [
        "provider_issue_mappings",
        "provider_health",
        "estimated_profit_bases",
        "provider_challenges",
    ] {
        assert!(index.has_table(table).unwrap(), "missing {table}");
        assert!(
            !index
                .columns(table)
                .unwrap()
                .iter()
                .any(|c| c.eq_ignore_ascii_case("pan")),
            "{table} may not contain PAN"
        );
    }
    let application_columns = index.columns("applications").unwrap();
    for required in ["source", "application_date", "created_at"] {
        assert!(application_columns.iter().any(|column| column == required));
    }
    assert!(
        !application_columns
            .iter()
            .any(|column| column.contains("pan"))
    );
}

#[test]
fn sqlite_lease_prevents_duplicate_job_ownership_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    {
        let index = LocalIndex::open(&path).unwrap();
        index.apply_event(&created()).unwrap();
        assert!(
            index
                .try_acquire_allotment_lease("job-1", "device-1", "token-a", 100, 200)
                .unwrap()
        );
        assert!(
            !index
                .try_acquire_allotment_lease("job-1", "device-1", "token-b", 101, 201)
                .unwrap()
        );
    }
    let index = LocalIndex::open(&path).unwrap();
    assert!(
        !index
            .try_acquire_allotment_lease("job-1", "device-1", "token-c", 150, 250)
            .unwrap()
    );
    assert!(
        index
            .try_acquire_allotment_lease("job-1", "device-1", "token-d", 201, 301)
            .unwrap()
    );
}

#[test]
fn retry_metadata_is_replayable() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let events = vec![
        created(),
        seal(
            "event-attempt",
            EventPayload::AllotmentAttemptStateUpdated {
                attempt_id: "attempt-1".into(),
                job_id: "job-1".into(),
                account_id: "account-1".into(),
                status: "RATE_LIMITED".into(),
                attempt_count: 2,
                allotted_lots: None,
                allotted_shares: None,
                source: "AUTOMATED".into(),
                provider_reference: None,
                safe_message: Some("provider asked us to retry later".into()),
                last_attempt_at: "100".into(),
                next_retry_at: Some("160".into()),
            },
        ),
    ];
    for event in &events {
        index.apply_event(event).unwrap();
    }
    let attempt = index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .unwrap();
    assert_eq!(attempt.attempt_count, 2);
    assert_eq!(attempt.next_retry_at.as_deref(), Some("160"));

    index.rebuild(&events).unwrap();
    let replayed = index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .unwrap();
    assert_eq!(replayed.attempt_count, 2);
    assert_eq!(replayed.next_retry_at.as_deref(), Some("160"));
}

#[test]
fn restart_expires_ephemeral_continuation_and_preserves_job() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    for event in [
        created(),
        seal(
            "event-running",
            EventPayload::AllotmentJobStatusChanged {
                job_id: "job-1".into(),
                status: "RUNNING".into(),
            },
        ),
        seal(
            "event-attempt-running",
            EventPayload::AllotmentAttemptStateUpdated {
                attempt_id: "attempt-1".into(),
                job_id: "job-1".into(),
                account_id: "account-1".into(),
                status: "NEEDS_HUMAN_VERIFICATION".into(),
                attempt_count: 1,
                allotted_lots: None,
                allotted_shares: None,
                source: "AUTOMATED".into(),
                provider_reference: None,
                safe_message: Some("verification required".into()),
                last_attempt_at: "100".into(),
                next_retry_at: None,
            },
        ),
        seal(
            "event-challenge",
            EventPayload::AllotmentProviderChallengeUpdated {
                challenge_id: "challenge-1".into(),
                job_id: "job-1".into(),
                attempt_id: "attempt-1".into(),
                account_id: "account-1".into(),
                provider_id: "bigshare-live".into(),
                challenge_type: "CAPTCHA".into(),
                status: "REQUIRED".into(),
                endpoint_id: "server-1".into(),
                continuation_reference: Some("continuation-1".into()),
                created_at: "2026-08-29T00:00:00Z".into(),
                expires_at: Some("2026-08-29T00:05:00Z".into()),
            },
        ),
    ] {
        index.apply_event(&event).unwrap();
    }

    index
        .reconcile_ephemeral_allotment_state_after_restart()
        .unwrap();

    let job = index.allotment_job_execution("job-1").unwrap().unwrap();
    assert_eq!(job.status, "VERIFICATION_REQUIRED_REFRESH");
    let attempt = index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .unwrap();
    assert_eq!(attempt.status, "VERIFICATION_REQUIRED_REFRESH");
    let challenge = index.provider_challenge("challenge-1").unwrap().unwrap();
    assert_eq!(challenge.status, "EXPIRED");
    assert_eq!(challenge.continuation_reference, None);
}
