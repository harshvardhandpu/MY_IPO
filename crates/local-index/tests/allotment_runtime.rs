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

fn attempt(status: &str) -> EventEnvelope {
    seal(
        "event-attempt",
        EventPayload::AllotmentAttemptStateUpdated {
            attempt_id: "attempt-1".into(),
            job_id: "job-1".into(),
            account_id: "account-1".into(),
            status: status.into(),
            attempt_count: 2,
            allotted_lots: (status == "ALLOTTED").then_some(1),
            allotted_shares: (status == "ALLOTTED").then_some(35),
            source: "AUTOMATED".into(),
            provider_reference: Some("provider-reference".into()),
            safe_message: Some("historical provider attempt".into()),
            last_attempt_at: "100".into(),
            next_retry_at: None,
        },
    )
}

fn fact(outcome: &str) -> EventEnvelope {
    fact_with("event-fact", outcome, "FIXTURE", "CONFIRMED_PROVIDER_RESPONSE")
}

fn fact_with(
    event_id: &str,
    outcome: &str,
    source: &str,
    provenance: &str,
) -> EventEnvelope {
    seal(
        event_id,
        EventPayload::AllotmentResolutionFactRecorded {
            fact_id: "fact-1".into(),
            job_id: "job-1".into(),
            account_id: "account-1".into(),
            outcome: outcome.into(),
            source: source.into(),
            allotted_lots: (outcome == "ALLOTTED").then_some(1),
            allotted_shares: (outcome == "ALLOTTED").then_some(35),
            provider_reference: Some("provider-reference".into()),
            provenance: provenance.into(),
            supersedes_attempt_id: Some("attempt-1".into()),
        },
    )
}

#[test]
fn schema_v9_adds_durable_runtime_tables_without_pan_columns() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    assert_eq!(index.schema_version().unwrap(), 9);
    for table in [
        "provider_issue_mappings",
        "provider_health",
        "estimated_profit_bases",
        "provider_challenges",
        "allotment_provider_attempts",
        "allotment_resolved_facts",
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
fn v8_migration_preserves_legacy_attempts_without_manufacturing_facts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    {
        let index = LocalIndex::open(&path).unwrap();
        index.apply_event(&created()).unwrap();
        index.apply_event(&attempt("ALLOTTED")).unwrap();
        let legacy = index
            .allotment_attempt("job-1", "account-1")
            .unwrap()
            .expect("legacy attempt");
        assert_eq!(legacy.status, "ALLOTTED");
    }

    // Simulate the installed schema-v8 projection: the legacy attempt table
    // remains, while the v9-only projections and migration marker are absent.
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE allotment_jobs SET status='COMPLETE_WITH_UNCONFIRMED' WHERE id='job-1'",
            [],
        )
        .unwrap();
    connection
        .execute_batch(
            "DROP TABLE allotment_provider_attempts;
             DROP TABLE allotment_resolved_facts;
             DELETE FROM schema_migrations WHERE version=9;",
        )
        .unwrap();
    drop(connection);

    let migrated = LocalIndex::open(&path).unwrap();
    assert_eq!(migrated.schema_version().unwrap(), 9);
    assert_eq!(
        migrated.list_allotment_jobs().unwrap()[0].5,
        "UNRESOLVED"
    );
    assert_eq!(migrated.allotment_attempt_history("job-1", "account-1").unwrap(), Vec::new());
    assert!(migrated
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_none());
    assert_eq!(
        migrated
            .allotment_attempt("job-1", "account-1")
            .unwrap()
            .expect("legacy attempt after migration")
            .status,
        "ALLOTTED"
    );
}

#[test]
fn explicit_authoritative_event_replays_to_fact_and_keeps_attempt_history() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let events = vec![created(), attempt("ALLOTTED"), fact("ALLOTTED")];
    index.rebuild(&events).unwrap();

    let history = index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, "ALLOTTED");
    let resolved = index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .expect("explicit fact");
    assert_eq!(resolved.outcome, "ALLOTTED");
    assert_eq!(resolved.state, "RESOLVED");
}

#[test]
fn resolution_fact_without_a_linked_attempt_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    index.rebuild(&[created(), fact("ALLOTTED")]).unwrap();
    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_none());
}

#[test]
fn cancellation_preserves_an_existing_resolved_fact_for_audit() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let cancelled = seal(
        "event-cancelled-after-fact",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "CANCELLED".into(),
        },
    );
    index
        .rebuild(&[created(), attempt("ALLOTTED"), fact("ALLOTTED")])
        .unwrap();
    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_some());
    index.apply_event(&cancelled).unwrap();
    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_some());
}

#[test]
fn tampered_event_is_rejected_before_projection_marker_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let original = created();
    let mut value = serde_json::to_value(&original).unwrap();
    value["content_hash"] = serde_json::Value::String("0".repeat(64));
    let tampered: EventEnvelope = serde_json::from_value(value).unwrap();

    let error = index.apply_event(&tampered).unwrap_err();
    assert!(error.to_string().contains("integrity"));
    assert!(index
        .projection_event_hash(original.event_id())
        .unwrap()
        .is_none());
}

#[test]
fn recommendation_replay_uses_a_stable_event_derived_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    let session = seal(
        "event-session-created",
        EventPayload::InvestmentSessionCreated {
            session_id: "session-recommendation".into(),
            actor_member_id: "owner-1".into(),
            declared_capital_paise: 100,
        },
    );
    let recommendation = seal(
        "event-recommendation",
        EventPayload::InvestmentRecommendationGenerated {
            session_id: "session-recommendation".into(),
            algorithm_version: "dev-v1".into(),
        },
    );
    let events = [session, recommendation];
    index.rebuild(&events).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    let first: Vec<(String, String, String)> = connection
        .prepare(
            "SELECT session_id, algorithm_version, recommendation_id
             FROM recommendations ORDER BY session_id, recommendation_id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    drop(connection);
    index.rebuild(&events).unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    let second: Vec<(String, String, String)> = connection
        .prepare(
            "SELECT session_id, algorithm_version, recommendation_id
             FROM recommendations ORDER BY session_id, recommendation_id",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].2, "event-recommendation");
}

#[test]
fn rebuild_ignores_unlinked_legacy_attempt_rows() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    index.apply_event(&created()).unwrap();
    index.apply_event(&attempt("PROVIDER_UNAVAILABLE")).unwrap();
    index.rebuild(&[created()]).unwrap();

    assert!(index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .is_none());
    assert!(index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap()
        .is_empty());
}

#[test]
fn rebuild_drops_unlinked_history_rows_across_repeated_rebuilds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    index.apply_event(&created()).unwrap();
    index.apply_event(&attempt("PROVIDER_UNAVAILABLE")).unwrap();
    drop(index);

    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("DELETE FROM allotment_attempts", [])
        .unwrap();
    drop(connection);

    let index = LocalIndex::open(&path).unwrap();
    index.rebuild(&[created()]).unwrap();
    let first = index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap();
    assert!(first.is_empty());

    index.rebuild(&[created()]).unwrap();
    let second = index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap();
    assert_eq!(second, first);
}

#[test]
fn rebuild_ignores_unlinked_cancellation_rows_without_manufacturing_fact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    {
        let index = LocalIndex::open(&path).unwrap();
        index.apply_event(&created()).unwrap();
        index.apply_event(&attempt("ALLOTTED")).unwrap();
    }
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE allotment_jobs SET status='CANCELLED', cancel_requested=1 WHERE id='job-1'",
                [],
            )
            .unwrap();
    }

    let index = LocalIndex::open(&path).unwrap();
    index.rebuild(&[created()]).unwrap();

    assert!(!index.allotment_job_is_cancelled("job-1").unwrap());
    assert!(index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .is_none());
    assert!(index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap()
        .is_empty());
    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_none());
}

#[test]
fn running_event_only_rebuilds_nonterminal_after_stale_sqlite_cancellation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    let running = seal(
        "event-running",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "RUNNING".into(),
        },
    );
    let cancelled = seal(
        "event-cancelled-stale",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "CANCELLED".into(),
        },
    );
    index.apply_event(&created()).unwrap();
    index.apply_event(&cancelled).unwrap();

    index.rebuild(&[created(), running]).unwrap();

    assert_eq!(index.list_allotment_jobs().unwrap()[0].5, "RUNNING");
    assert!(!index.allotment_job_is_cancelled("job-1").unwrap());
}

#[test]
fn stale_unlinked_attempt_is_not_restored_under_running_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    index.apply_event(&created()).unwrap();
    index.apply_event(&attempt("CANCELLED")).unwrap();
    let running = seal(
        "event-running-attempt-control",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "RUNNING".into(),
        },
    );
    index.rebuild(&[created(), running]).unwrap();
    assert!(index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .is_none());
    assert!(index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap()
        .is_empty());
}

#[test]
fn production_rebuild_excludes_fixture_jobs_from_live_projection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    index.apply_event(&created()).unwrap();
    index.apply_event(&attempt("PROVIDER_UNAVAILABLE")).unwrap();
    index
        .apply_event(&seal(
            "event-challenge",
            EventPayload::AllotmentProviderChallengeUpdated {
                challenge_id: "challenge-1".into(),
                job_id: "job-1".into(),
                attempt_id: "attempt-1".into(),
                account_id: "account-1".into(),
                provider_id: "kfintech-fixture".into(),
                challenge_type: "CAPTCHA".into(),
                status: "PRESENTED".into(),
                endpoint_id: "fixture".into(),
                continuation_reference: None,
                created_at: "100".into(),
                expires_at: None,
            },
        ))
        .unwrap();
    index
        .rebuild_production(&[created(), attempt("PROVIDER_UNAVAILABLE")])
        .unwrap();

    assert!(index.list_allotment_jobs().unwrap().is_empty());
    assert!(index
        .allotment_attempt("job-1", "account-1")
        .unwrap()
        .is_none());
    assert!(index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap()
        .is_empty());
    assert!(index.provider_challenge("challenge-1").unwrap().is_none());
}

#[test]
fn duplicate_event_id_with_different_hash_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    let original = created();
    index.apply_event(&original).unwrap();
    let forged = EventEnvelope::seal(NewEvent {
        event_id: "event-created".into(),
        aggregate_type: "allotment_job".into(),
        aggregate_id: "job-1".into(),
        aggregate_revision: 99,
        actor_member_id: "owner-1".into(),
        device_id: "device-1".into(),
        occurred_at: "2026-01-01T00:00:01Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "RUNNING".into(),
        },
    })
    .unwrap();
    assert_ne!(original.content_hash(), forged.content_hash());
    assert_eq!(
        index
            .projection_event_hash("event-created")
            .unwrap()
            .as_deref(),
        Some(original.content_hash())
    );
    assert!(index.apply_event(&forged).is_err());
    assert!(!index.allotment_job_is_cancelled("job-1").unwrap());
}

#[test]
fn replayed_cancellation_does_not_restore_legacy_attempt_as_current_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("index.sqlite3");
    let index = LocalIndex::open(&path).unwrap();
    index.apply_event(&created()).unwrap();
    index.apply_event(&attempt("ALLOTTED")).unwrap();

    let cancelled = seal(
        "event-cancelled",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "CANCELLED".into(),
        },
    );
    index.rebuild(&[created(), attempt("ALLOTTED"), cancelled]).unwrap();

    assert_eq!(
        index
            .allotment_attempt("job-1", "account-1")
            .unwrap()
            .expect("cancelled attempt")
            .status,
        "CANCELLED"
    );
    let history = index
        .allotment_attempt_history("job-1", "account-1")
        .unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, "ALLOTTED");
}

#[test]
fn forged_resolution_fact_is_rejected_at_replay_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    index
        .rebuild(&[
            created(),
            attempt("ALLOTTED"),
            fact_with(
                "event-forged-provider",
                "ALLOTTED",
                "FORGED_PROVIDER",
                "CONFIRMED_PROVIDER_RESPONSE",
            ),
            fact_with(
                "event-forged-manual",
                "NOT_ALLOTTED",
                "MANUAL",
                "OWNER_REPORTED_MANUAL",
            ),
        ])
        .unwrap();

    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_none());
}

#[test]
fn cancelled_replay_blocks_later_fact_and_completion_but_keeps_attempt_history() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let cancelled = seal(
        "event-cancelled",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "CANCELLED".into(),
        },
    );
    let completed = seal(
        "event-completed-after-cancel",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "COMPLETE".into(),
        },
    );
    index
        .rebuild(&[
            created(),
            cancelled,
            attempt("ALLOTTED"),
            fact("ALLOTTED"),
            completed,
        ])
        .unwrap();

    assert!(index.allotment_job_is_cancelled("job-1").unwrap());
    assert_eq!(index.list_allotment_jobs().unwrap()[0].5, "CANCELLED");
    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_none());
    assert_eq!(
        index
            .allotment_attempt_history("job-1", "account-1")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn late_cancellation_does_not_delete_completed_resolution_fact() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let completed = seal(
        "event-completed-before-cancel",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "COMPLETE".into(),
        },
    );
    let cancelled = seal(
        "event-cancelled-after-complete",
        EventPayload::AllotmentJobStatusChanged {
            job_id: "job-1".into(),
            status: "CANCELLED".into(),
        },
    );
    index
        .rebuild(&[
            created(),
            attempt("ALLOTTED"),
            fact("ALLOTTED"),
            completed,
            cancelled,
        ])
        .unwrap();

    assert_eq!(index.list_allotment_jobs().unwrap()[0].5, "COMPLETE");
    assert!(index
        .resolved_allotment_fact("job-1", "account-1")
        .unwrap()
        .is_some());
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
