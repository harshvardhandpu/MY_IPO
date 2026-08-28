//! Phase 2C: the multi-IPO sanitiized decision request (master-source §78.2).

use sanket_intelligence_vault::{AirBoundaryError, InvestmentDecisionRequest, PlannedIpo};

fn ok() -> InvestmentDecisionRequest {
    InvestmentDecisionRequest::new(
        "session-1",
        120_000_000,
        vec![PlannedIpo {
            typed_name: "Example IPO Limited".to_owned(),
            planned_amount_per_account_paise: 1_500_000,
        }],
        "ipo-ranking-v001",
    )
}

#[test]
fn request_round_trips() {
    let r = ok();
    let json = serde_json::to_string(&r).unwrap();
    let back: InvestmentDecisionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(r, back);
}

#[test]
fn request_rejects_pan_in_typed_name() {
    let mut r = ok();
    r.ipos[0].typed_name = "ABCDE1234F".to_owned();
    assert!(matches!(
        r.assert_safe(),
        Err(AirBoundaryError::ProhibitedToken("pan"))
    ));
}

#[test]
fn request_rejects_upi_in_typed_name() {
    let mut r = ok();
    r.ipos[0].typed_name = "member123@okhdfcbank".to_owned();
    assert!(matches!(
        r.assert_safe(),
        Err(AirBoundaryError::ProhibitedToken("upi"))
    ));
}

#[test]
fn request_has_no_private_field_names() {
    let json = serde_json::to_string(&ok()).unwrap();
    for forbidden in [
        "pan",
        "upi",
        "member_name",
        "friend_name",
        "email",
        "proof",
        "member_vault",
        "vault_path",
    ] {
        assert!(!json.contains(forbidden), "must not contain {forbidden}");
    }
}

#[test]
fn request_exposes_only_approved_keys() {
    let json = serde_json::to_string(&ok()).unwrap();
    for expected in [
        "session_id",
        "declared_daily_capital_paise",
        "account_count",
        "typed_name",
        "planned_amount_per_account_paise",
        "algorithm_version",
    ] {
        assert!(json.contains(expected), "must contain {expected}");
    }
}

#[test]
fn request_account_count_is_derived_not_identities() {
    // account_count is a plain integer — it can never carry identities.
    let r = InvestmentDecisionRequest::new("s", 1, vec![], "v1");
    assert_eq!(r.account_count, 0);
    let r2 = InvestmentDecisionRequest::with_account_count("s", 1, vec![], "v1", 6);
    assert_eq!(r2.account_count, 6);
}
