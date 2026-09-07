use sanket_domain::{EventEnvelope, EventPayload, NewEvent, Role};

fn event(payload: EventPayload) -> EventEnvelope {
    EventEnvelope::seal(NewEvent {
        event_id: "auth-event-1".into(),
        aggregate_type: "auth".into(),
        aggregate_id: "account-1".into(),
        aggregate_revision: 1,
        actor_member_id: "owner-1".into(),
        device_id: "device-1".into(),
        occurred_at: "100".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload,
    })
    .unwrap()
}

#[test]
fn authentication_events_serialize_metadata_without_reusable_secrets() {
    let payload = EventPayload::OwnerBootstrapped {
        account_id: "owner-1".into(),
        email: "owner@example.invalid".into(),
        verifier_version: 1,
        verifier: "argon2id$v=19$m=19456,t=2,p=1$saltsalt$hash".into(),
    };
    let json = serde_json::to_string(&event(payload)).unwrap();
    assert!(json.contains("OWNER_BOOTSTRAPPED"));
    assert!(!json.contains("correct horse battery staple"));
    assert!(!json.contains("invite-secret"));
}

#[test]
fn role_admin_can_manage_auth_but_core_member_cannot() {
    assert!(Role::Owner.can_manage_members());
    assert!(Role::Admin.can_manage_members());
    assert!(!Role::CoreMember.can_manage_members());
}

#[test]
fn invite_event_contains_digest_and_expiry_only() {
    let json = serde_json::to_string(&event(EventPayload::InviteIssued {
        invite_id: "invite-1".into(),
        account_id: "account-1".into(),
        email: "member@example.invalid".into(),
        invite_digest: "sha256:deadbeef".into(),
        expires_at: "200".into(),
    }))
    .unwrap();
    assert!(json.contains("INVITE_ISSUED"));
    assert!(json.contains("sha256:deadbeef"));
    assert!(!json.contains("invite-secret"));
}
