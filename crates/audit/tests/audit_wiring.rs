//! End-to-end wiring: purpose-scoped access → audit → sealed event → vault.

use sanket_audit::{AuditContext, access_audit_event};
use sanket_identity_security::{
    IdentityCipher, IdentityKey, InMemoryKeyProvider, Pan, SensitiveIdentityRecord,
    SensitiveIdentityService, SensitivePurpose,
};
use sanket_member_vault::MemberVault;

fn key() -> IdentityKey {
    IdentityKey::from_bytes(&[5u8; 32])
}

fn synthetic_pan() -> String {
    ["TESTP", "1234", "Z"].concat()
}

#[test]
fn access_audit_flows_into_vault_event_without_pan() {
    let pan = synthetic_pan();
    let cipher = IdentityCipher::new(key());
    let record = SensitiveIdentityRecord::encrypt_pan(
        Pan::parse(&pan).expect("valid"),
        "member-1",
        &cipher,
        "kid-1",
    )
    .expect("encrypt");

    // 1. Purpose-scoped access emits an audit.
    let provider = InMemoryKeyProvider::new("kid-1", key());
    let mut service = SensitiveIdentityService::new(provider);
    service
        .with_pan(
            &record,
            SensitivePurpose::AllotmentCheck,
            "member-1",
            |_| (),
        )
        .expect("access");
    let audit = service.take_last_audit().expect("audit emitted");

    // 2. The audit converts into a sealable event.
    let context = AuditContext {
        event_id: "evt-audit-1".into(),
        actor_member_id: "member-1".into(),
        device_id: "device-1".into(),
        occurred_at: audit.occurred_at.clone(),
        app_version: "0.1.0".into(),
    };
    let event = access_audit_event(&audit, context).expect("seal");
    assert_eq!(event.event_type(), "SENSITIVE_IDENTITY_ACCESSED");

    // 3. The event persists in the MemberVault.
    let root = std::env::temp_dir().join(format!("sanket-audit-wire-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let vault = MemberVault::open(&root).expect("open");
    vault.append_event(&event).expect("append");

    // 4. The persisted event carries the purpose, never the PAN.
    let bytes = std::fs::read(root.join("_events/evt-audit-1.json")).expect("read");
    let text = String::from_utf8(bytes).expect("utf8");
    assert!(text.contains("ALLOTMENT_CHECK"));
    assert!(text.contains("member-1"));
    assert!(!text.contains(&pan));

    let _ = std::fs::remove_dir_all(&root);
}
