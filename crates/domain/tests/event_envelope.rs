use sanket_domain::{EventEnvelope, EventPayload, NewEvent};

fn device_event() -> NewEvent {
    NewEvent {
        event_id: "event-01".into(),
        aggregate_type: "DEVICE".into(),
        aggregate_id: "device-01".into(),
        aggregate_revision: 1,
        actor_member_id: "member-01".into(),
        device_id: "device-01".into(),
        occurred_at: "2026-08-28T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::DeviceRegistered {
            device_label: "Synthetic Linux device".into(),
        },
    }
}

#[test]
fn sealed_event_verifies_its_content_hash() {
    let envelope = EventEnvelope::seal(device_event()).expect("event should seal");

    assert_eq!("DEVICE_REGISTERED", envelope.event_type());
    assert!(envelope.verify_integrity().expect("hash should verify"));
    assert!(envelope.content_hash().starts_with("sha256:"));
}

#[test]
fn changed_event_content_fails_integrity_check() {
    let envelope = EventEnvelope::seal(device_event()).expect("event should seal");
    let mut serialized = serde_json::to_value(envelope).expect("event should serialize");
    serialized["aggregate_revision"] = serde_json::json!(2);
    let changed: EventEnvelope = serde_json::from_value(serialized).expect("shape remains valid");

    assert!(!changed.verify_integrity().expect("hash check should run"));
}
