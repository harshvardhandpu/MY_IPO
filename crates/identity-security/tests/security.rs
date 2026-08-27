use sanket_identity_security::{
    IdentityCipher, IdentityKey, InMemoryKeyProvider, KeyProvider, Pan, Redacted,
    SensitiveIdentityRecord, SensitiveIdentityService, SensitivePurpose,
};

fn sample_key() -> IdentityKey {
    IdentityKey::from_bytes(&[7u8; 32])
}

fn sample_pan() -> Pan {
    Pan::parse("ABCDE1234F").expect("valid PAN")
}

// ---------- PAN ----------

#[test]
fn pan_normalizes_whitespace_and_case() {
    let lower = Pan::parse("abcde1234f").expect("valid");
    let spaced = Pan::parse(" ABCDE 1234F ").expect("valid");
    assert_eq!(lower.as_normalized(), "ABCDE1234F");
    assert_eq!(spaced.as_normalized(), "ABCDE1234F");
}

#[test]
fn pan_rejects_invalid_formats() {
    assert!(Pan::parse("").is_err());
    assert!(Pan::parse("ABCDE1234").is_err()); // too short
    assert!(Pan::parse("ABCDE1234FG").is_err()); // too long
    assert!(Pan::parse("ABCDE12345").is_err()); // wrong final char
    assert!(Pan::parse("ABCDE123$F").is_err()); // symbol in numeric block
    assert!(Pan::parse("abcde1234F").is_ok()); // valid after normalize
}

#[test]
fn pan_masks_to_abcde_first_last() {
    let pan = sample_pan();
    let masked = pan.mask();
    assert_eq!(masked.to_string(), "ABCDE****F");
    assert_eq!(pan.to_string(), "ABCDE****F");
}

#[test]
fn pan_debug_is_redacted() {
    let pan = sample_pan();
    let debug = format!("{pan:?}");
    assert!(!debug.contains("ABCDE1234F"));
    assert!(!debug.contains("1234"));
}

#[test]
fn masked_pan_display_never_reveals_full_pan() {
    let masked = sample_pan().mask();
    assert_eq!(masked.to_string(), "ABCDE****F");
    assert!(!masked.to_string().contains("1234"));
}

// ---------- Redacted ----------

#[test]
fn redacted_never_reveals_secret() {
    let secret = Redacted::new("topsecretvalue".to_string());
    assert_eq!(format!("{secret}"), "[REDACTED]");
    assert_eq!(format!("{secret:?}"), "[REDACTED]");
    assert_eq!(secret.expose(), "topsecretvalue");
}

// ---------- Crypto ----------

#[test]
fn cipher_round_trips() {
    let cipher = IdentityCipher::new(sample_key());
    let plaintext = b"{\"pan\":\"ABCDE1234F\"}";
    let envelope = cipher.encrypt(plaintext, "kid-1").expect("encrypt");
    let recovered = cipher.decrypt(&envelope).expect("decrypt");
    assert_eq!(recovered, plaintext);
    assert_eq!(envelope.key_id(), "kid-1");
    assert_eq!(envelope.algorithm(), "XChaCha20-Poly1305");
}

#[test]
fn cipher_wrong_key_fails() {
    let envelope = IdentityCipher::new(sample_key())
        .encrypt(b"secret", "kid-1")
        .expect("encrypt");
    let other = IdentityCipher::new(IdentityKey::from_bytes(&[9u8; 32]));
    assert!(matches!(
        other.decrypt(&envelope),
        Err(sanket_identity_security::CipherError::Authentication)
    ));
}

#[test]
fn cipher_corrupted_ciphertext_fails() {
    let mut envelope = IdentityCipher::new(sample_key())
        .encrypt(b"secret", "kid-1")
        .expect("encrypt");
    let mut ct = envelope.ciphertext.clone();
    ct[0] ^= 0xff;
    envelope.ciphertext = ct;
    assert!(
        IdentityCipher::new(sample_key())
            .decrypt(&envelope)
            .is_err()
    );
}

#[test]
fn cipher_tampered_nonce_fails() {
    let mut envelope = IdentityCipher::new(sample_key())
        .encrypt(b"secret", "kid-1")
        .expect("encrypt");
    envelope.nonce = vec![1u8; 24];
    assert!(
        IdentityCipher::new(sample_key())
            .decrypt(&envelope)
            .is_err()
    );
}

#[test]
fn envelope_version_is_rejected_when_unsupported() {
    let mut envelope = IdentityCipher::new(sample_key())
        .encrypt(b"secret", "kid-1")
        .expect("encrypt");
    envelope.version = 99;
    assert!(matches!(
        IdentityCipher::new(sample_key()).decrypt(&envelope),
        Err(sanket_identity_security::CipherError::UnsupportedVersion(_))
    ));
}

// ---------- Key provider ----------

#[test]
fn in_memory_key_provider_round_trips() {
    let provider = InMemoryKeyProvider::new("kid-1", sample_key());
    let fetched = provider.key("kid-1").expect("present");
    assert_eq!(fetched.as_bytes(), &[7u8; 32]);
    assert!(provider.key("missing").is_none());
}

// ---------- Sensitive identity ----------

#[test]
fn sensitive_record_masks_pan_and_hides_secret() {
    let cipher = IdentityCipher::new(sample_key());
    let record = SensitiveIdentityRecord::encrypt_pan(sample_pan(), "member-1", &cipher, "kid-1")
        .expect("encrypt");
    assert_eq!(record.masked_pan().to_string(), "ABCDE****F");
    let debug = format!("{record:?}");
    assert!(!debug.contains("1234"));
    assert!(!debug.contains("ABCDE1234F"));
}

#[test]
fn service_with_pan_allows_allotment_check() {
    let provider = InMemoryKeyProvider::new("kid-1", sample_key());
    let mut service = SensitiveIdentityService::new(provider);
    let cipher = IdentityCipher::new(sample_key());
    let record = SensitiveIdentityRecord::encrypt_pan(sample_pan(), "member-1", &cipher, "kid-1")
        .expect("encrypt");

    let seen = service
        .with_pan(
            &record,
            SensitivePurpose::AllotmentCheck,
            "member-1",
            |pan| pan.to_owned(),
        )
        .expect("authorized");

    assert_eq!(seen, "ABCDE1234F");
}

#[test]
fn service_rejects_unknown_purpose() {
    let provider = InMemoryKeyProvider::new("kid-1", sample_key());
    let mut service = SensitiveIdentityService::new(provider);
    let cipher = IdentityCipher::new(sample_key());
    let record = SensitiveIdentityRecord::encrypt_pan(sample_pan(), "member-1", &cipher, "kid-1")
        .expect("encrypt");

    let result = service.with_pan(&record, SensitivePurpose::Unknown, "member-1", |_| ());
    assert!(result.is_err());
}

#[test]
fn service_generates_audit_without_pan() {
    let provider = InMemoryKeyProvider::new("kid-1", sample_key());
    let mut service = SensitiveIdentityService::new(provider);
    let cipher = IdentityCipher::new(sample_key());
    let record = SensitiveIdentityRecord::encrypt_pan(sample_pan(), "member-1", &cipher, "kid-1")
        .expect("encrypt");

    service
        .with_pan(
            &record,
            SensitivePurpose::AllotmentCheck,
            "member-1",
            |_| (),
        )
        .expect("authorized");

    let audit = service.take_last_audit().expect("audit present");
    assert_eq!(audit.account_id, "member-1");
    assert_eq!(audit.purpose, "ALLOTMENT_CHECK");
    let serialized = serde_json::to_string(&audit).expect("serialize");
    assert!(!serialized.contains("1234"));
    assert!(!serialized.contains("ABCDE1234F"));
}

#[test]
fn secret_does_not_escape_service_without_closed_callback() {
    // The plaintext PAN type is not reachable outside the closure: `with_pan`
    // returns the closure result, not the PAN. Compiled-in: there is no accessor
    // that returns a `&str` PAN from the service.
    let provider = InMemoryKeyProvider::new("kid-1", sample_key());
    let service = SensitiveIdentityService::new(provider);
    // Structurally: service has no `get_pan`/`pan()` method — enforced by type.
    let _ = service; // no PAN accessor exists to call
}

// ---------- AI boundary / leakage ----------

#[test]
fn masked_pan_serialization_has_no_plaintext() {
    let serialized = serde_json::to_string(&sample_pan().mask()).expect("serialize");
    assert!(!serialized.contains("1234"));
    assert!(!serialized.contains("ABCDE1234F"));
}

#[test]
fn envelope_serialization_has_no_plaintext_pan() {
    let envelope = IdentityCipher::new(sample_key())
        .encrypt(b"ABCDE1234F", "kid-1")
        .expect("encrypt");
    let json = serde_json::to_string(&envelope).expect("serialize");
    assert!(!json.contains("ABCDE1234F"));
    assert!(!json.contains("1234"));
}
