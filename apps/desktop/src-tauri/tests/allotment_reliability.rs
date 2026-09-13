//! Deterministic regressions for unresolved allotment semantics.
//!
//! These tests use the development-synthetic MUFG path. No live provider
//! request and no real PAN lookup is performed: the authorization gate returns
//! a typed provider-unavailable result before identity access.

use std::env;
use std::fs;
use std::path::PathBuf;

use sanket_domain::{EventEnvelope, EventPayload, NewEvent};
use sanket_desktop_lib::service::{
    Application, HistoricalApplicationRequest, ManualAllotmentRequest, OnboardMemberRequest,
    StartAllotmentRequest, SubmitIpoInput, SubmitRequest,
};
use sanket_local_index::LocalIndex;
use sanket_member_vault::MemberVault;
use uuid::Uuid;

#[test]
#[ignore = "explicit live projection migration; requires both live paths"]
fn migrate_explicit_live_projection_from_vault() {
    let index_path = env::var("SANKET_LIVE_INDEX").expect("SANKET_LIVE_INDEX");
    let vault_path = env::var("SANKET_LIVE_VAULT").expect("SANKET_LIVE_VAULT");
    let index_path = PathBuf::from(index_path);
    let vault_path = PathBuf::from(vault_path);
    let before_attempts = rusqlite::Connection::open(&index_path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM allotment_attempts", [], |row| row.get::<_, i64>(0))
        .unwrap();

    let vault = MemberVault::open(&vault_path).unwrap();
    let mut events = vault.list_events().unwrap();
    events.sort_by(|left, right| {
        left.occurred_at()
            .cmp(right.occurred_at())
            .then_with(|| left.event_id().cmp(right.event_id()))
    });
    let explicit_fact_events = events
        .iter()
        .filter(|event| {
            matches!(
                event.payload(),
                EventPayload::AllotmentResolutionFactRecorded { .. }
            )
        })
        .count() as i64;

    let index = LocalIndex::open(&index_path).unwrap();
    index.rebuild_production(&events).unwrap();
    assert_eq!(index.schema_version().unwrap(), 9);

    let connection = rusqlite::Connection::open(&index_path).unwrap();
    let after_attempts = connection
        .query_row("SELECT COUNT(*) FROM allotment_attempts", [], |row| row.get::<_, i64>(0))
        .unwrap();
    let after_attempt_history = connection
        .query_row(
            "SELECT COUNT(*) FROM allotment_provider_attempts",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    let resolved_facts = connection
        .query_row("SELECT COUNT(*) FROM allotment_resolved_facts", [], |row| row.get::<_, i64>(0))
        .unwrap();
    let old_unconfirmed = connection
        .query_row(
            "SELECT COUNT(*) FROM allotment_jobs WHERE status='COMPLETE_WITH_UNCONFIRMED'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    assert!(after_attempts + after_attempt_history >= before_attempts);
    assert!(resolved_facts <= explicit_fact_events);
    assert_eq!(old_unconfirmed, 0);

    let app = Application::new("live-migration-readback".into(), vault_path, index_path).unwrap();
    let mut symbiotec_seen = false;
    let mut symbiotec_application_id = None;
    for candidate in app.list_allotment_candidates().unwrap() {
        if candidate.ipo_name.to_ascii_lowercase().contains("symbiotec") {
            symbiotec_seen = true;
            symbiotec_application_id = Some(candidate.application_id.clone());
            assert!(candidate.account_count > 0);
            assert_ne!(candidate.overall_job_state, "COMPLETE_WITH_UNCONFIRMED");
            assert_ne!(candidate.overall_job_state, "RUNNING");
            assert!(candidate.final_count <= candidate.account_count);
        }
    }
    assert!(symbiotec_seen, "fresh Symbiotec candidate readback");
    let symbiotec_application_id = symbiotec_application_id.expect("Symbiotec application id");
    assert!(!events.iter().any(|event| {
        matches!(
            event.payload(),
            EventPayload::LookupAuthorizationConsumed { application_id, .. }
                if application_id == &symbiotec_application_id
        )
    }));
}

fn tmp() -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("sanket-allotment-reliability-{}", Uuid::now_v7()));
    fs::create_dir_all(&path).expect("test root");
    path
}

fn queued_mufg_job() -> (PathBuf, PathBuf, Application, String, String) {
    let root = tmp();
    let index_path = root.join("index.sqlite3");
    let member_id = format!("member-{}", Uuid::now_v7());
    let session_id = format!("session-{}", Uuid::now_v7());
    let app = Application::new(
        "reliability-test-device".into(),
        root.join("vault"),
        index_path.clone(),
    )
    .expect("application");
    app.onboard_member(OnboardMemberRequest {
        member_id: member_id.clone(),
        display_name: "Owner".into(),
        email: "owner@example.invalid".into(),
        role: "OWNER".into(),
        primary_account_label: Some("Primary".into()),
        broker: None,
        upi_id: "owner@upi".into(),
        pan: "ABCDE1234A".into(),
        consented: true,
    })
    .expect("owner");
    app.submit(SubmitRequest {
        session_id,
        actor_member_id: member_id.clone(),
        declared_capital_paise: 100_000,
        recommendation_id: None,
        ipos: vec![SubmitIpoInput {
            name: "Symbiotec Pharmalab Limited".into(),
            amount_paise: 1_482_000,
            account_ids: vec![member_id.clone()],
            registrar_id: "mufg_intime".into(),
            expected_allotment_date: None,
            metadata_snapshot: None,
            confirm_metadata_changes: false,
        }],
    })
    .expect("submitted historical-shaped application");
    let candidate = app
        .list_allotment_candidates()
        .expect("candidate")
        .into_iter()
        .next()
        .expect("one candidate");
    let queued = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: candidate.application_id,
            session_id: candidate.session_id,
            ipo_name: candidate.ipo_name,
            actor_member_id: member_id.clone(),
            registrar_id: Some(candidate.registrar_id),
        })
        .expect("queued");
    (root, index_path, app, queued.job_id, member_id)
}

#[test]
fn mufg_provider_unavailable_after_retry_exhaustion_is_not_complete() {
    let root = tmp();
    let index_path = root.join("index.sqlite3");
    let member_id = format!("member-{}", Uuid::now_v7());
    let session_id = format!("session-{}", Uuid::now_v7());
    let app = Application::new(
        "reliability-test-device".into(),
        root.join("vault"),
        index_path.clone(),
    )
    .expect("application");

    app.onboard_member(OnboardMemberRequest {
        member_id: member_id.clone(),
        display_name: "Owner".into(),
        email: "owner@example.invalid".into(),
        role: "OWNER".into(),
        primary_account_label: Some("Primary".into()),
        broker: None,
        upi_id: "owner@upi".into(),
        pan: "ABCDE1234A".into(),
        consented: true,
    })
    .expect("owner");
    app.submit(SubmitRequest {
        session_id: session_id.clone(),
        actor_member_id: member_id.clone(),
        declared_capital_paise: 100_000,
        recommendation_id: None,
        ipos: vec![SubmitIpoInput {
            name: "Symbiotec Pharmalab Limited".into(),
            amount_paise: 1_482_000,
            account_ids: vec![member_id.clone()],
            registrar_id: "mufg_intime".into(),
            expected_allotment_date: None,
            metadata_snapshot: None,
            confirm_metadata_changes: false,
        }],
    })
    .expect("submitted historical-shaped application");

    let candidate = app
        .list_allotment_candidates()
        .expect("candidate")
        .into_iter()
        .next()
        .expect("one candidate");
    let queued = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: candidate.application_id,
            session_id: candidate.session_id,
            ipo_name: candidate.ipo_name,
            actor_member_id: member_id.clone(),
            registrar_id: Some(candidate.registrar_id),
        })
        .expect("queued");

    let first = app
        .run_allotment_job_once(&queued.job_id)
        .expect("first attempt executes");
    assert!(first);
    let first_report = app
        .get_allotment_report(&queued.job_id)
        .expect("first report");
    assert_eq!(first_report.status, "RETRYABLE_PROVIDER_FAILURE");
    assert_eq!(first_report.final_count, 0);
    assert_eq!(first_report.pending_count, 1);
    assert_eq!(first_report.accounts[0].status, "PROVIDER_UNAVAILABLE");
    assert!(first_report.accounts[0].next_retry_at.is_some());

    // Make the deterministic retry due without waiting for wall-clock backoff.
    let connection = rusqlite::Connection::open(&index_path).expect("raw index");
    connection
        .execute(
            "UPDATE allotment_attempts SET next_retry_at='0' WHERE job_id=?1",
            [&queued.job_id],
        )
        .expect("retry due");
    drop(connection);

    let second = app
        .run_allotment_job_once(&queued.job_id)
        .expect("retry executes");
    assert!(second);
    let exhausted = app
        .get_allotment_report(&queued.job_id)
        .expect("exhausted report");
    assert_eq!(exhausted.status, "UNRESOLVED");
    assert_eq!(exhausted.final_count, 0);
    assert_eq!(exhausted.pending_count, 1);
    assert_eq!(exhausted.accounts[0].status, "PROVIDER_UNAVAILABLE");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn manual_result_requires_explicit_result_and_official_source() {
    let (root, _index_path, app, job_id, member_id) = queued_mufg_job();
    let error = app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("NOT_ALLOTTED".into()),
        allotted_lots: None,
        allotted_shares: None,
        explicit_not_allotted: false,
        official_source: None,
        note: None,
    });
    assert!(
        error.is_err(),
        "negative manual result needs official provenance"
    );

    let unresolved = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: job_id.clone(),
            account_id: member_id.clone(),
            actor_member_id: member_id.clone(),
            result: Some("COULD_NOT_VERIFY".into()),
            allotted_lots: None,
            allotted_shares: None,
            explicit_not_allotted: false,
            official_source: None,
            note: None,
        })
        .expect("unresolved manual result");
    assert_eq!(unresolved.status, "UNKNOWN");
    assert_eq!(unresolved.resolution_state, "UNRESOLVED");

    let allotted = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: job_id.clone(),
            account_id: member_id.clone(),
            actor_member_id: member_id,
            result: Some("ALLOTTED".into()),
            allotted_lots: Some(1),
            allotted_shares: Some(35),
            explicit_not_allotted: false,
            official_source: Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into()),
            note: None,
        })
        .expect("allotted manual result");
    assert_eq!(allotted.status, "ALLOTTED");
    assert_eq!(allotted.resolution_state, "MANUAL_CONFIRMED");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "COMPLETE");
    assert_eq!(report.final_count, 1);
    assert_eq!(report.pending_count, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn authoritative_reconciliation_preserves_failed_provider_history() {
    let (root, index_path, app, job_id, member_id) = queued_mufg_job();
    app.run_allotment_job_once(&job_id).expect("first attempt");
    let connection = rusqlite::Connection::open(&index_path).expect("raw index");
    connection
        .execute(
            "UPDATE allotment_attempts SET next_retry_at='0' WHERE job_id=?1",
            [&job_id],
        )
        .expect("retry due");
    drop(connection);
    app.run_allotment_job_once(&job_id).expect("second attempt");

    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(1),
        allotted_shares: Some(35),
        explicit_not_allotted: false,
        official_source: Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into()),
        note: None,
    })
    .expect("authoritative reconciliation");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "COMPLETE");
    assert_eq!(report.accounts[0].resolution_state, "MANUAL_CONFIRMED");
    assert_eq!(report.accounts[0].status, "ALLOTTED");
    assert_eq!(
        report.accounts[0]
            .attempt_history
            .iter()
            .filter(|attempt| attempt.status == "PROVIDER_UNAVAILABLE")
            .count(),
        2
    );
    let index = LocalIndex::open(&index_path).expect("index");
    let history = index
        .allotment_attempt_history(&job_id, &member_id)
        .expect("immutable history");
    assert_eq!(
        history.len(),
        4,
        "queued state, two provider attempts, and manual evidence"
    );
    assert_eq!(
        history
            .iter()
            .filter(|attempt| attempt.status == "PROVIDER_UNAVAILABLE")
            .count(),
        2
    );
    let fact = index
        .resolved_allotment_fact(&job_id, &member_id)
        .expect("resolved fact lookup")
        .expect("resolved fact");
    assert_eq!(fact.outcome, "ALLOTTED");
    assert_eq!(fact.state, "RESOLVED");
    assert_eq!(fact.provenance, "OFFICIAL_MANUAL_SOURCE");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn historical_mufg_issue_mapping_is_projected_for_lookup() {
    let root = tmp();
    let index_path = root.join("index.sqlite3");
    let member_id = format!("member-{}", Uuid::now_v7());
    let app = Application::new(
        "reliability-test-device".into(),
        root.join("vault"),
        index_path.clone(),
    )
    .expect("application");
    app.onboard_member(OnboardMemberRequest {
        member_id: member_id.clone(),
        display_name: "Owner".into(),
        email: "owner@example.invalid".into(),
        role: "OWNER".into(),
        primary_account_label: Some("Primary".into()),
        broker: None,
        upi_id: "owner@upi".into(),
        pan: "ABCDE1234A".into(),
        consented: true,
    })
    .expect("owner");
    let historical = app
        .record_historical_application(HistoricalApplicationRequest {
            actor_member_id: member_id.clone(),
            account_id: member_id,
            ipo_name: "Symbiotec Pharmalab Limited".into(),
            amount_paise: 1_482_000,
            application_date: Some("2026-08-01".into()),
            registrar_id: "mufg_intime".into(),
            provider_issue_id: "11926".into(),
            owner_affirmed: true,
        })
        .expect("historical application");
    let index = LocalIndex::open(&index_path).expect("index");
    let mapping = index
        .provider_issue_mapping(&historical.application_id, &historical.provider_id)
        .expect("mapping lookup")
        .expect("historical mapping");
    assert_eq!(mapping.provider_issue_id, "11926");
    assert_eq!(mapping.registrar_id, "mufg_intime");
    assert_eq!(mapping.ipo_name, "Symbiotec Pharmalab Limited");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn conflicting_authoritative_results_require_review() {
    let (root, index_path, app, job_id, member_id) = queued_mufg_job();
    let source = Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into());
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("NOT_ALLOTTED".into()),
        allotted_lots: None,
        allotted_shares: None,
        explicit_not_allotted: false,
        official_source: source.clone(),
        note: None,
    })
    .expect("not allotted");
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(1),
        allotted_shares: Some(35),
        explicit_not_allotted: false,
        official_source: source,
        note: None,
    })
    .expect("allotted");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "CONFLICT_REVIEW_REQUIRED");
    assert_eq!(
        report.accounts[0].resolution_state,
        "CONFLICT_REVIEW_REQUIRED"
    );
    assert_eq!(report.final_count, 0);
    assert_eq!(report.pending_count, 1);
    let index = LocalIndex::open(&index_path).expect("index");
    let fact = index
        .resolved_allotment_fact(&job_id, &member_id)
        .expect("conflict fact lookup")
        .expect("conflict fact");
    assert_eq!(fact.state, "CONFLICT_REVIEW_REQUIRED");
    assert_eq!(fact.outcome, "NOT_ALLOTTED");
    assert_eq!(fact.conflicting_outcome.as_deref(), Some("ALLOTTED"));
    let _ = fs::remove_dir_all(root);
}

fn overwrite_latest_attempt(
    index_path: &PathBuf,
    job_id: &str,
    status: &str,
    allotted_lots: Option<u32>,
    allotted_shares: Option<u64>,
) {
    let connection = rusqlite::Connection::open(index_path).expect("raw index");
    connection
        .execute(
            "UPDATE allotment_attempts
             SET status=?1, allotted_lots=?2, allotted_shares=?3
             WHERE job_id=?4",
            rusqlite::params![
                status,
                allotted_lots.map(i64::from),
                allotted_shares.map(|value| value as i64),
                job_id,
            ],
        )
        .expect("overwrite legacy attempt");
}

#[allow(clippy::too_many_arguments)]
fn apply_authoritative_fact(
    index_path: &std::path::Path,
    event_id: &str,
    fact_id: &str,
    job_id: &str,
    account_id: &str,
    outcome: &str,
    allotted_lots: Option<u32>,
    allotted_shares: Option<u64>,
    source: &str,
    provenance: &str,
) {
    let connection = rusqlite::Connection::open(index_path).expect("raw index");
    let (attempt_id, application_id, provider_id): (String, String, String) = connection
        .query_row(
            "SELECT attempts.id, jobs.application_id, jobs.provider_id
             FROM allotment_attempts AS attempts
             JOIN allotment_jobs AS jobs ON jobs.id=attempts.job_id
             WHERE attempts.job_id=?1 AND attempts.account_id=?2
             ORDER BY attempts.rowid ASC LIMIT 1",
            rusqlite::params![job_id, account_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("stable attempt and job identity");
    drop(connection);
    let attempt_event = EventEnvelope::seal(NewEvent {
        event_id: format!("{event_id}-attempt"),
        aggregate_type: "allotment".into(),
        aggregate_id: job_id.into(),
        aggregate_revision: 99,
        actor_member_id: "owner".into(),
        device_id: "test-device".into(),
        occurred_at: "2026-09-09T00:00:00Z".into(),
        app_version: "0.1.2-test".into(),
        previous_event_hash: None,
        payload: EventPayload::AllotmentAttemptStateUpdated {
            attempt_id: attempt_id.clone(),
            job_id: job_id.into(),
            account_id: account_id.into(),
            status: outcome.into(),
            attempt_count: 1,
            allotted_lots,
            allotted_shares,
            provider_reference: Some("provider-reference".into()),
            safe_message: Some("confirmed".into()),
            source: source.into(),
            last_attempt_at: "2026-09-09T00:00:00Z".into(),
            next_retry_at: None,
        },
    })
    .expect("attempt event");
    let fact_event = EventEnvelope::seal(NewEvent {
        event_id: event_id.into(),
        aggregate_type: "allotment".into(),
        aggregate_id: job_id.into(),
        aggregate_revision: 100,
        actor_member_id: "owner".into(),
        device_id: "test-device".into(),
        occurred_at: "2026-09-09T00:00:00Z".into(),
        app_version: "0.1.2-test".into(),
        previous_event_hash: None,
        payload: EventPayload::AllotmentResolutionFactRecorded {
            fact_id: fact_id.into(),
            job_id: job_id.into(),
            account_id: account_id.into(),
            outcome: outcome.into(),
            source: source.into(),
            allotted_lots,
            allotted_shares,
            provider_reference: Some("provider-reference".into()),
            provenance: provenance.into(),
            supersedes_attempt_id: Some(attempt_id),
        },
    })
    .expect("fact event");
    let index = LocalIndex::open(index_path).expect("index");
    let discovery_event = EventEnvelope::seal(NewEvent {
        event_id: format!("{event_id}-issue"),
        aggregate_type: "allotment_provider".into(),
        aggregate_id: application_id.clone(),
        aggregate_revision: 98,
        actor_member_id: "owner".into(),
        device_id: "test-device".into(),
        occurred_at: "2026-09-09T00:00:00Z".into(),
        app_version: "0.1.2-test".into(),
        previous_event_hash: None,
        payload: EventPayload::AllotmentProviderDiscovered {
            application_id,
            registrar_id: "mufg_intime".into(),
            provider_id,
            provider_issue_id: "11926".into(),
            ipo_name: "Symbiotec Pharmalab Limited".into(),
            official_status_url: "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into(),
            last_verified_at: "2026-09-09T00:00:00Z".into(),
        },
    })
    .expect("issue discovery event");
    index.apply_event(&discovery_event).expect("apply issue discovery");
    index.apply_event(&attempt_event).expect("apply attempt");
    index.apply_event(&fact_event).expect("apply authoritative fact");
}

#[test]
fn legacy_allotted_attempt_without_fact_is_unresolved_and_not_counted() {
    let (root, index_path, app, job_id, _member_id) = queued_mufg_job();
    overwrite_latest_attempt(&index_path, &job_id, "ALLOTTED", Some(1), Some(35));

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "UNRESOLVED");
    assert_eq!(report.final_count, 0);
    assert_eq!(report.pending_count, 1);
    assert_eq!(report.accounts[0].status, "UNRESOLVED");
    assert_eq!(report.accounts[0].resolution_state, "UNRESOLVED");
    assert_eq!(report.accounts[0].allotted_lots, None);
    assert_eq!(report.accounts[0].allotted_shares, None);

    let candidate = app
        .list_allotment_candidates()
        .expect("candidate")
        .into_iter()
        .next()
        .expect("one candidate");
    assert_eq!(candidate.final_count, 0);
    assert_eq!(candidate.pending_count, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn legacy_not_allotted_attempt_without_fact_is_unresolved_and_not_counted() {
    let (root, index_path, app, job_id, _member_id) = queued_mufg_job();
    overwrite_latest_attempt(&index_path, &job_id, "NOT_ALLOTTED", None, None);

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "UNRESOLVED");
    assert_eq!(report.final_count, 0);
    assert_eq!(report.pending_count, 1);
    assert_eq!(report.accounts[0].status, "UNRESOLVED");
    assert_eq!(report.accounts[0].resolution_state, "UNRESOLVED");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn authoritative_fact_is_the_only_source_of_final_state_after_legacy_attempt() {
    let (root, index_path, app, job_id, member_id) = queued_mufg_job();
    overwrite_latest_attempt(&index_path, &job_id, "ALLOTTED", Some(9), Some(999));

    let recorded = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: job_id.clone(),
            account_id: member_id.clone(),
            actor_member_id: member_id,
            result: Some("NOT_ALLOTTED".into()),
            allotted_lots: None,
            allotted_shares: None,
            explicit_not_allotted: false,
            official_source: Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into()),
            note: None,
        })
        .expect("authoritative manual result");
    assert_eq!(recorded.resolution_state, "MANUAL_CONFIRMED");
    assert_eq!(recorded.status, "NOT_ALLOTTED");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "COMPLETE");
    assert_eq!(report.final_count, 1);
    assert_eq!(report.accounts[0].status, "NOT_ALLOTTED");
    assert_eq!(report.accounts[0].resolution_state, "MANUAL_CONFIRMED");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn same_outcome_different_lots_requires_conflict_review() {
    let (root, _index_path, app, job_id, member_id) = queued_mufg_job();
    let source = Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into());
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(1),
        allotted_shares: Some(35),
        explicit_not_allotted: false,
        official_source: source.clone(),
        note: None,
    })
    .expect("first authoritative result");
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(2),
        allotted_shares: Some(35),
        explicit_not_allotted: false,
        official_source: source,
        note: None,
    })
    .expect("second authoritative result");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "CONFLICT_REVIEW_REQUIRED");
    assert_eq!(
        report.accounts[0].resolution_state,
        "CONFLICT_REVIEW_REQUIRED"
    );
    assert_eq!(report.final_count, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn same_outcome_different_shares_requires_conflict_review() {
    let (root, _index_path, app, job_id, member_id) = queued_mufg_job();
    let source = Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into());
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(1),
        allotted_shares: Some(35),
        explicit_not_allotted: false,
        official_source: source.clone(),
        note: None,
    })
    .expect("first authoritative result");
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id.clone(),
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(1),
        allotted_shares: Some(36),
        explicit_not_allotted: false,
        official_source: source,
        note: None,
    })
    .expect("second authoritative result");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "CONFLICT_REVIEW_REQUIRED");
    assert_eq!(
        report.accounts[0].resolution_state,
        "CONFLICT_REVIEW_REQUIRED"
    );
    assert_eq!(report.final_count, 0);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn authoritative_quantity_missing_on_one_side_preserves_richer_evidence() {
    let (root, index_path, app, job_id, member_id) = queued_mufg_job();
    apply_authoritative_fact(
        &index_path,
        "provider-fact-one",
        "fact-one",
        &job_id,
        &member_id,
        "ALLOTTED",
        None,
        Some(35),
        "AUTOMATED_PROVIDER",
        "CONFIRMED_PROVIDER_RESPONSE",
    );
    apply_authoritative_fact(
        &index_path,
        "provider-fact-two",
        "fact-two",
        &job_id,
        &member_id,
        "ALLOTTED",
        Some(1),
        None,
        "AUTOMATED_PROVIDER",
        "CONFIRMED_PROVIDER_RESPONSE",
    );

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "COMPLETE");
    assert_eq!(report.final_count, 1);
    assert_eq!(report.accounts[0].resolution_state, "FINAL_ALLOTTED");
    assert_eq!(report.accounts[0].allotted_lots, Some(1));
    assert_eq!(report.accounts[0].allotted_shares, Some(35));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn same_outcome_and_quantities_are_not_a_conflict() {
    let (root, index_path, app, job_id, member_id) = queued_mufg_job();
    apply_authoritative_fact(
        &index_path,
        "same-fact-one",
        "same-fact-id-one",
        &job_id,
        &member_id,
        "ALLOTTED",
        Some(1),
        Some(35),
        "AUTOMATED_PROVIDER",
        "CONFIRMED_PROVIDER_RESPONSE",
    );
    apply_authoritative_fact(
        &index_path,
        "same-fact-two",
        "same-fact-id-two",
        &job_id,
        &member_id,
        "ALLOTTED",
        Some(1),
        Some(35),
        "AUTOMATED_PROVIDER",
        "CONFIRMED_PROVIDER_RESPONSE",
    );
    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "COMPLETE");
    assert_eq!(report.accounts[0].resolution_state, "FINAL_ALLOTTED");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn manual_and_provider_quantity_disagreement_requires_conflict_review() {
    let (root, index_path, app, job_id, member_id) = queued_mufg_job();
    apply_authoritative_fact(
        &index_path,
        "provider-fact-before-manual",
        "provider-fact-before-manual-id",
        &job_id,
        &member_id,
        "ALLOTTED",
        Some(1),
        Some(35),
        "AUTOMATED_PROVIDER",
        "CONFIRMED_PROVIDER_RESPONSE",
    );
    app.record_manual_allotment_result(ManualAllotmentRequest {
        job_id: job_id.clone(),
        account_id: member_id.clone(),
        actor_member_id: member_id,
        result: Some("ALLOTTED".into()),
        allotted_lots: Some(2),
        allotted_shares: Some(70),
        explicit_not_allotted: false,
        official_source: Some("https://in.mpms.mufg.com/Initial_Offer/IPO.aspx".into()),
        note: None,
    })
    .expect("manual authoritative result");

    let report = app.get_allotment_report(&job_id).expect("report");
    assert_eq!(report.status, "CONFLICT_REVIEW_REQUIRED");
    assert_eq!(
        report.accounts[0].resolution_state,
        "CONFLICT_REVIEW_REQUIRED"
    );
    assert_eq!(report.final_count, 0);
    let _ = fs::remove_dir_all(root);
}
