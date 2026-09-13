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

#[test]
fn unsafe_event_ids_are_rejected_when_sealing_or_deserializing() {
    for unsafe_id in ["../outside", "nested/event", "nested\\event", ".", ".."] {
        let mut new_event = device_event();
        new_event.event_id = unsafe_id.into();
        assert!(
            EventEnvelope::seal(new_event).is_err(),
            "seal should reject {unsafe_id:?}"
        );

        let envelope = EventEnvelope::seal(device_event()).expect("seal valid event");
        let mut serialized = serde_json::to_value(envelope).expect("serialize valid event");
        serialized["event_id"] = serde_json::json!(unsafe_id);
        assert!(
            serde_json::from_value::<EventEnvelope>(serialized).is_err(),
            "deserialize should reject {unsafe_id:?}"
        );
    }
}

#[test]
fn legacy_ipo_event_without_metadata_keeps_its_hash() {
    let event = NewEvent {
        event_id: "event-ipo-01".into(),
        aggregate_type: "APPLICATION".into(),
        aggregate_id: "application-01".into(),
        aggregate_revision: 1,
        actor_member_id: "member-01".into(),
        device_id: "device-01".into(),
        occurred_at: "2026-08-28T00:00:00Z".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: EventPayload::IpoApplicationCreated {
            application_id: "application-01".into(),
            session_id: "session-01".into(),
            ipo_name: "Example IPO".into(),
            planned_amount_paise: 100,
            registrar_id: "kfintech".into(),
            registrar_name: "KFintech".into(),
            official_status_url: Some("https://example.test/status".into()),
            expected_allotment_date: Some("2026-09-01".into()),
            source: "OWNER_CURRENT_ENTRY".into(),
            application_date: None,
            metadata: None,
        },
    };
    let envelope = EventEnvelope::seal(event).expect("event should seal");
    let mut legacy = serde_json::to_value(envelope).expect("event should serialize");
    legacy["payload"]
        .as_object_mut()
        .unwrap()
        .remove("metadata");
    let decoded: EventEnvelope =
        serde_json::from_value(legacy).expect("legacy event should decode");

    assert!(
        decoded
            .verify_integrity()
            .expect("legacy hash should verify")
    );
}

#[test]
fn lookup_authorization_event_names_and_payload_are_safe() {
    let granted = EventPayload::LookupAuthorizationGranted {
        authorization_id: "lookup-auth-1".into(),
        application_id: "application-1".into(),
        provider_id: "mufg-intime-live".into(),
        expiry_time: "1788211200".into(),
    };
    let consumed = EventPayload::LookupAuthorizationConsumed {
        authorization_id: "lookup-auth-1".into(),
        application_id: "application-1".into(),
        provider_id: "mufg-intime-live".into(),
        execution_id: "execution-1".into(),
        account_ids: vec!["account-1".into()],
        timestamp: "1788210900".into(),
    };

    let granted_envelope = EventEnvelope::seal(NewEvent {
        event_id: "lookup-granted-1".into(),
        aggregate_type: "lookup_authorization".into(),
        aggregate_id: "lookup-auth-1".into(),
        aggregate_revision: 1,
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: "1788210900".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: granted.clone(),
    })
    .expect("seal granted");
    let consumed_envelope = EventEnvelope::seal(NewEvent {
        event_id: "lookup-consumed-1".into(),
        aggregate_type: "lookup_authorization".into(),
        aggregate_id: "lookup-auth-1".into(),
        aggregate_revision: 2,
        actor_member_id: "SYSTEM".into(),
        device_id: "device-1".into(),
        occurred_at: "1788210901".into(),
        app_version: "0.1.0".into(),
        previous_event_hash: None,
        payload: consumed.clone(),
    })
    .expect("seal consumed");

    assert_eq!(
        granted_envelope.event_type(),
        "LOOKUP_AUTHORIZATION_GRANTED"
    );
    assert_eq!(
        consumed_envelope.event_type(),
        "LOOKUP_AUTHORIZATION_CONSUMED"
    );
    let json = serde_json::to_string(&(granted, consumed)).expect("serialize");
    assert!(!json.to_ascii_lowercase().contains("pan"));
    assert!(!json.to_ascii_lowercase().contains("token"));
    assert!(!json.to_ascii_lowercase().contains("credential"));
}
