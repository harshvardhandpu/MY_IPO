//! Phase 2B: IdentitySecret carries PAN + UPI inside the encrypted envelope.

use sanket_identity_security::{
    IdentityCipher, IdentityKey, IdentitySecret, InMemoryKeyProvider, Pan, SensitiveIdentityRecord,
    SensitiveIdentityService, SensitivePurpose,
};

fn key() -> IdentityKey {
    IdentityKey::from_bytes(&[3u8; 32])
}

fn cipher() -> IdentityCipher {
    IdentityCipher::new(key())
}

fn synthetic_pan() -> String {
    format!("{}1234{}", "ABCDE", "F")
}

#[test]
fn identity_secret_round_trips_pan_and_upi() {
    let secret = IdentitySecret {
        pan: Pan::parse(&synthetic_pan()).expect("valid"),
        upi_id: Some("member123@okhdfcbank".to_owned()),
    };
    let record = SensitiveIdentityRecord::encrypt_identity(secret, "member-1", &cipher(), "kid-1")
        .expect("encrypt");

    let provider = InMemoryKeyProvider::new("kid-1", key());
    let mut service = SensitiveIdentityService::new(provider);

    let mut seen_upi: Option<String> = None;
    let seen_pan = service
        .with_identity(&record, SensitivePurpose::AllotmentCheck, "member-1", |s| {
            seen_upi = s.upi_id.clone();
            s.pan.as_normalized().to_owned()
        })
        .expect("access");

    assert_eq!(seen_pan, synthetic_pan());
    assert_eq!(seen_upi.as_deref(), Some("member123@okhdfcbank"));
}

#[test]
fn with_pan_still_works_on_identity_envelope() {
    let secret = IdentitySecret {
        pan: Pan::parse(&synthetic_pan()).expect("valid"),
        upi_id: None,
    };
    let record = SensitiveIdentityRecord::encrypt_identity(secret, "member-1", &cipher(), "kid-1")
        .expect("encrypt");

    let provider = InMemoryKeyProvider::new("kid-1", key());
    let mut service = SensitiveIdentityService::new(provider);
    let seen = service
        .with_pan(&record, SensitivePurpose::AllotmentCheck, "member-1", |p| {
            p.to_owned()
        })
        .expect("access");
    assert_eq!(seen, synthetic_pan());
}

#[test]
fn encrypt_pan_keeps_working_without_upi() {
    let record = SensitiveIdentityRecord::encrypt_pan(
        Pan::parse(&synthetic_pan()).expect("valid"),
        "member-1",
        &cipher(),
        "kid-1",
    )
    .expect("encrypt");

    let provider = InMemoryKeyProvider::new("kid-1", key());
    let mut service = SensitiveIdentityService::new(provider);
    let mut saw_upi: Option<Option<String>> = None;
    service
        .with_identity(&record, SensitivePurpose::AllotmentCheck, "member-1", |s| {
            saw_upi = Some(s.upi_id.clone());
        })
        .expect("access");
    assert_eq!(saw_upi, Some(None));
}

#[test]
fn identity_secret_envelope_hides_upi_in_serialization() {
    let secret = IdentitySecret {
        pan: Pan::parse(&synthetic_pan()).expect("valid"),
        upi_id: Some("member123@okhdfcbank".to_owned()),
    };
    let record = SensitiveIdentityRecord::encrypt_identity(secret, "member-1", &cipher(), "kid-1")
        .expect("encrypt");
    let json = serde_json::to_string(record.envelope()).expect("serialize");
    assert!(!json.contains("member123@okhdfcbank"));
    assert!(!json.contains(&synthetic_pan()));
}

#[test]
fn upi_validation_rejects_obviously_invalid_ids() {
    assert!(IdentitySecret::validate_upi("").is_err());
    assert!(IdentitySecret::validate_upi("no-at-sign").is_err());
    assert!(IdentitySecret::validate_upi("a@b@c").is_err());
    assert!(IdentitySecret::validate_upi(" has space@bank").is_err());
    assert!(IdentitySecret::validate_upi("member123@okhdfcbank").is_ok());
}
