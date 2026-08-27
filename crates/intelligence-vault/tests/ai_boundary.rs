use sanket_intelligence_vault::{AirBoundaryError, InvestmentDecisionPayload};

fn safe_payload() -> InvestmentDecisionPayload {
    InvestmentDecisionPayload {
        ipo_name: "Example Limited".to_owned(),
        ipo_identifier: "EXLIM-2026".to_owned(),
        price_band_paise_low: 100_000,
        price_band_paise_high: 120_000,
        lot_size: 50,
        subscription_times: 12,
        listing_gain_basis_points: 1_500,
        public_context_hashes: vec!["sha256:deadbeef".to_owned()],
    }
}

#[test]
fn safe_payload_passes_boundary() {
    assert!(safe_payload().assert_safe().is_ok());
}

#[test]
fn pan_in_public_context_hash_is_rejected() {
    let mut p = safe_payload();
    p.public_context_hashes.push("ABCDE1234F".to_owned());
    assert!(matches!(
        p.assert_safe(),
        Err(AirBoundaryError::ProhibitedToken("pan"))
    ));
}

#[test]
fn pan_in_ipo_identifier_is_rejected() {
    let mut p = safe_payload();
    p.ipo_identifier = "ABCDE1234F".to_owned();
    assert!(matches!(
        p.assert_safe(),
        Err(AirBoundaryError::ProhibitedToken("pan"))
    ));
}

#[test]
fn upi_in_ipo_name_is_rejected() {
    let mut p = safe_payload();
    p.ipo_name = "member123@okhdfcbank".to_owned();
    assert!(matches!(
        p.assert_safe(),
        Err(AirBoundaryError::ProhibitedToken("upi"))
    ));
}

#[test]
fn member_and_friend_names_are_not_representable_fields() {
    // The public payload has no field that can carry a member name, friend name,
    // proof bytes, MemberVault path, or any private financial object. This is a
    // structural guarantee: the only fields are sanitized scalars and hashes.
    //
    // We assert it here by confirming the type exposes none of those field names
    // via serde — i.e. serializing yields only the sanitized keys.
    let json = serde_json::to_string(&safe_payload()).expect("serialize");
    for forbidden in [
        "member_name",
        "friend_name",
        "pan",
        "upi",
        "proof",
        "member_vault",
        "vault_path",
    ] {
        assert!(
            !json.contains(forbidden),
            "payload must not contain field {forbidden}"
        );
    }
}

#[test]
fn payload_round_trips_serde() {
    let p = safe_payload();
    let json = serde_json::to_string(&p).expect("serialize");
    let back: InvestmentDecisionPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(p, back);
}
