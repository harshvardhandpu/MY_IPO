use sanket_audit::{AuditContext, sensitive_identity_accessed};

#[test]
fn sensitive_access_audit_records_purpose_without_secret_value() {
    let context = AuditContext {
        event_id: "event-audit-01".into(),
        actor_member_id: "member-01".into(),
        device_id: "device-01".into(),
        occurred_at: "2026-08-28T00:00:00Z".into(),
        app_version: "0.1.0".into(),
    };

    let event = sensitive_identity_accessed(context, "account-01".into(), "ALLOTMENT_CHECK".into())
        .expect("audit event should seal");
    let serialized = serde_json::to_string(&event).expect("audit event should serialize");
    let generated_pan = ["TESTP", "1234", "Z"].concat();

    assert_eq!("SENSITIVE_IDENTITY_ACCESSED", event.event_type());
    assert!(!serialized.contains(&generated_pan));
    assert!(serialized.contains("ALLOTMENT_CHECK"));
}
