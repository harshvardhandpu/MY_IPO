//! Phase 2C(3A): investment session, IPO, application, and allocation domain.

use sanket_domain::{
    BasisPoints, EventPayload, InvestmentAllocation, InvestmentSession, IpoApplication, Money,
    NewEvent,
};

#[test]
fn session_requires_positive_declared_capital() {
    assert!(InvestmentSession::open("s1", "member-1", Money::from_paise(0)).is_err());
    assert!(InvestmentSession::open("s1", "member-1", Money::from_rupees(15_000)).is_ok());
}

#[test]
fn session_captures_actor_and_capital() {
    let s = InvestmentSession::open("s1", "member-1", Money::from_rupees(15_000)).unwrap();
    assert_eq!(s.actor_member_id(), "member-1");
    assert_eq!(s.declared_capital().paise(), 1_500_000);
    assert_eq!(s.status(), "OPEN");
}

#[test]
fn session_submitted_transitions_state() {
    let mut s = InvestmentSession::open("s1", "member-1", Money::from_rupees(15_000)).unwrap();
    s.mark_submitted();
    assert_eq!(s.status(), "SUBMITTED");
}

#[test]
fn session_void_requires_submitted_and_excludes_open() {
    let mut open = InvestmentSession::open("s1", "member-1", Money::from_rupees(15_000)).unwrap();
    assert!(open.mark_voided().is_err());
    open.mark_submitted();
    open.mark_voided().unwrap();
    assert_eq!(open.status(), "VOIDED");
    assert!(open.is_voided());
    assert!(!open.is_submitted());
}

#[test]
fn application_requires_nonempty_ipo_name_and_positive_amount() {
    // Empty name rejected.
    let app = IpoApplication::create("a1", "s1", "", Money::from_paise(1));
    assert!(app.is_err());
    // Zero amount rejected.
    let app = IpoApplication::create("a1", "s1", "Example IPO", Money::from_paise(0));
    assert!(app.is_err());
    // Valid.
    let app = IpoApplication::create("a1", "s1", "Example IPO", Money::from_rupees(2_000));
    assert!(app.is_ok());
}

#[test]
fn allocation_references_account_not_pan() {
    let alloc = InvestmentAllocation::new(
        "alloc-1",
        "app-1",
        "account-1",
        Money::from_rupees(2_000),
        BasisPoints::friend_share_default(),
    );
    // The allocation holds account_id, never anything PAN-shaped.
    assert_eq!(alloc.account_id(), "account-1");
    assert_eq!(alloc.amount().paise(), 200_000);
    assert_eq!(alloc.share_basis_points().value(), 1_000);
}

#[test]
fn session_created_event_seals() {
    let payload = EventPayload::InvestmentSessionCreated {
        session_id: "s1".to_owned(),
        actor_member_id: "member-1".to_owned(),
        declared_capital_paise: 1_500_000,
    };
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("INVESTMENT_SESSION_CREATED"));
    assert!(json.contains("1500000"));
}

#[test]
fn allocation_added_event_seals_without_pan() {
    let payload = EventPayload::AllocationAdded {
        allocation_id: "alloc-1".to_owned(),
        application_id: "app-1".to_owned(),
        account_id: "account-1".to_owned(),
        amount_paise: 200_000,
        share_basis_points: 1_000,
    };
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("ALLOCATION_ADDED"));
    // No PAN-like token can appear.
    assert!(!json.contains("1234"));
}

#[test]
fn recommendation_generated_and_applied_events_seal() {
    let generated = EventPayload::InvestmentRecommendationGenerated {
        session_id: "s1".to_owned(),
        algorithm_version: "dev-ranking-v001".to_owned(),
    };
    let applied = EventPayload::InvestmentRecommendationApplied {
        session_id: "s1".to_owned(),
        recommendation_id: "rec-1".to_owned(),
    };
    assert!(
        serde_json::to_string(&generated)
            .unwrap()
            .contains("INVESTMENT_RECOMMENDATION_GENERATED")
    );
    assert!(
        serde_json::to_string(&applied)
            .unwrap()
            .contains("INVESTMENT_RECOMMENDATION_APPLIED")
    );
}

#[test]
fn session_submitted_event_seals() {
    let payload = EventPayload::InvestmentSessionSubmitted {
        session_id: "s1".to_owned(),
        recommendation_id: None,
    };
    let json = serde_json::to_string(&payload).unwrap();
    assert!(json.contains("INVESTMENT_SESSION_SUBMITTED"));
}

#[test]
fn new_event_uses_uuidv7_when_unset() {
    // If a caller omits an id, the domain generates a stable UUIDv7 id.
    let event = NewEvent {
        event_id: String::new(),
        aggregate_type: "session".to_owned(),
        aggregate_id: "s1".to_owned(),
        aggregate_revision: 1,
        actor_member_id: "member-1".to_owned(),
        device_id: "device-1".to_owned(),
        occurred_at: "2026-08-28T00:00:00Z".to_owned(),
        app_version: "0.1.0".to_owned(),
        previous_event_hash: None,
        payload: EventPayload::InvestmentSessionCreated {
            session_id: "s1".to_owned(),
            actor_member_id: "member-1".to_owned(),
            declared_capital_paise: 1_500_000,
        },
    };
    // The seal fills an empty event_id with a UUIDv7 (36 chars, starts with a hex version nibble).
    let sealed = sanket_domain::EventEnvelope::seal(event).unwrap();
    assert_eq!(sealed.event_id().len(), 36);
}
