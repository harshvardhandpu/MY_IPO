//! Phase 2C: investment projection tables + rebuild-from-events.

use sanket_domain::{EventEnvelope, EventPayload, NewEvent};
use sanket_local_index::LocalIndex;

fn seal(
    event_id: &str,
    aggregate_type: &str,
    aggregate_id: &str,
    payload: EventPayload,
) -> EventEnvelope {
    EventEnvelope::seal(NewEvent {
        event_id: event_id.to_owned(),
        aggregate_type: aggregate_type.to_owned(),
        aggregate_id: aggregate_id.to_owned(),
        aggregate_revision: 1,
        actor_member_id: "member-1".to_owned(),
        device_id: "device-1".to_owned(),
        occurred_at: "2026-08-28T00:00:00Z".to_owned(),
        app_version: "0.1.0".to_owned(),
        previous_event_hash: None,
        payload,
    })
    .unwrap()
}

#[test]
fn schema_version_two_adds_projection_tables() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    assert!(index.schema_version().unwrap() >= 2);
    for table in [
        "members",
        "friend_accounts",
        "ipos",
        "investment_sessions",
        "allocations",
        "recommendations",
    ] {
        assert!(index.has_table(table).unwrap(), "missing table {table}");
    }
}

#[test]
fn projection_tables_have_no_full_pan_column() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    for table in [
        "members",
        "friend_accounts",
        "investment_sessions",
        "allocations",
    ] {
        let cols = index.columns(table).unwrap();
        // Only `masked_pan` may mention PAN; no column may store a full PAN.
        assert!(
            !cols
                .iter()
                .any(|c| c.to_lowercase().contains("pan") && c != "masked_pan"),
            "{table} must have no full-PAN column: {cols:?}"
        );
    }
}

#[test]
fn upsert_member_and_friend_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();

    index
        .upsert_member("member-1", "Sanket", "OWNER", "ABCDE****F")
        .unwrap();
    index
        .upsert_friend("friend-1", "member-1", "Broker", "ABCDE****F", 1_000)
        .unwrap();

    let members = index.list_members().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].0, "member-1");
    assert_eq!(members[0].3, "ABCDE****F"); // masked only (id, name, role, masked_pan)

    let friends = index.list_active_friends().unwrap();
    assert_eq!(friends.len(), 1);
    assert_eq!(friends[0].0, "friend-1");
}

#[test]
fn rebuild_session_and_allocations_from_events() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();

    // Persist a session + application + allocation via events.
    let events = vec![
        seal(
            "e1",
            "session",
            "s1",
            EventPayload::InvestmentSessionCreated {
                session_id: "s1".to_owned(),
                actor_member_id: "member-1".to_owned(),
                declared_capital_paise: 1_500_000,
            },
        ),
        seal(
            "e2",
            "application",
            "app-1",
            EventPayload::IpoApplicationCreated {
                application_id: "app-1".to_owned(),
                session_id: "s1".to_owned(),
                ipo_name: "Example IPO".to_owned(),
                planned_amount_paise: 200_000,
                registrar_id: "kfintech".into(),
                registrar_name: "KFintech".into(),
                official_status_url: Some("https://ipostatus.kfintech.com".into()),
                expected_allotment_date: None,
            },
        ),
        seal(
            "e3",
            "allocation",
            "alloc-1",
            EventPayload::AllocationAdded {
                allocation_id: "alloc-1".to_owned(),
                application_id: "app-1".to_owned(),
                account_id: "account-1".to_owned(),
                amount_paise: 200_000,
                share_basis_points: 1_000,
            },
        ),
    ];

    for event in &events {
        index.apply_event(event).unwrap();
    }

    // Sessions projected.
    let sessions = index.list_sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].0, "s1");
    assert_eq!(sessions[0].2, 1_500_000);

    // Allocations projected with account id, no PAN.
    let allocations = index.list_allocations("s1").unwrap();
    assert_eq!(allocations.len(), 1);
    assert_eq!(allocations[0].2, "account-1");

    // Rebuild from scratch (drop + replay) and verify identical projection.
    index.rebuild(&events).unwrap();
    let sessions2 = index.list_sessions().unwrap();
    assert_eq!(sessions2.len(), 1);
    assert_eq!(sessions2[0].0, "s1");
    let allocations2 = index.list_allocations("s1").unwrap();
    assert_eq!(allocations2.len(), 1);
    assert_eq!(allocations2[0].2, "account-1");
}

#[test]
fn apply_event_replays_idempotently() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let event = seal(
        "e1",
        "session",
        "s1",
        EventPayload::InvestmentSessionCreated {
            session_id: "s1".to_owned(),
            actor_member_id: "member-1".to_owned(),
            declared_capital_paise: 1_500_000,
        },
    );
    index.apply_event(&event).unwrap();
    index.apply_event(&event).unwrap(); // idempotent: same event_id twice OK
    assert_eq!(index.list_sessions().unwrap().len(), 1);
}

#[test]
fn recommendation_event_stores_algorithm_version() {
    let dir = tempfile::tempdir().unwrap();
    let index = LocalIndex::open(&dir.path().join("index.sqlite3")).unwrap();
    let event = seal(
        "e1",
        "session",
        "s1",
        EventPayload::InvestmentRecommendationGenerated {
            session_id: "s1".to_owned(),
            algorithm_version: "dev-ranking-v001".to_owned(),
        },
    );
    index.apply_event(&event).unwrap();
    let recs = index.list_recommendations().unwrap();
    assert_eq!(recs.len(), 1);
    assert_eq!(recs[0].1, "dev-ranking-v001"); // (session_id, algorithm_version, recommendation_id)
}
