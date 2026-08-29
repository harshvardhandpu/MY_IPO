use sanket_allotment::{
    AllotmentLookupContext, AllotmentProvider, KfintechProvider, NormalizedAllotmentStatus,
    ProviderHealth, ProviderResultProvenance, RegistrarIssue, SanitizedFixtureProvenance,
};
use sanket_identity_security::Pan;

fn parse_case(name: &str) -> String {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/kfintech/cases.json")).unwrap();
    serde_json::to_string(&cases[name]).unwrap()
}

fn parse_result(
    body: &str,
) -> Result<sanket_allotment::ProviderAllotmentResult, sanket_allotment::ProviderError> {
    KfintechProvider::parse_result_body(
        body,
        true,
        "2026-08-28T18:09:00Z",
        "kfin-result-data-array-v1",
    )
}

#[test]
fn offline_live_never_not_allotted() {
    let p = KfintechProvider::offline_for_tests();
    let pan = Pan::parse("ABCDE1234A").unwrap();
    let ctx = AllotmentLookupContext {
        job_id: "j".into(),
        attempt_id: "a".into(),
        account_id: "c".into(),
        issue: RegistrarIssue {
            registrar_id: "kfintech".into(),
            registrar_name: "KFintech".into(),
            official_status_url: Some("https://ipostatus.kfintech.com".into()),
            issue_code: None,
            ipo_name: "X".into(),
        },
    };
    let err = p.check_allotment(&ctx, &pan).unwrap_err();
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn human_gate_status() {
    let p = KfintechProvider::human_gate_for_tests();
    assert_eq!(p.health(), ProviderHealth::HumanVerificationRequired);
}

#[test]
fn discovers_current_bundle_issue_records() {
    let fixture = include_str!("fixtures/kfintech/issues.js");
    let provenance = SanitizedFixtureProvenance::from_json(include_str!(
        "fixtures/kfintech/issues.js.provenance.json"
    ))
    .expect("valid provenance");
    provenance
        .verify_content(fixture.as_bytes())
        .expect("fixture hash");

    let issues = KfintechProvider::parse_issue_bundle(fixture, "2026-08-28T18:09:00Z")
        .expect("recognized issue bundle");
    assert_eq!(issues.len(), 2);
    assert_eq!(issues[0].provider_issue_id, "90000000001");
    assert_eq!(issues[0].display_name, "SYNTHETIC ALPHA LIMITED");
}

#[test]
fn rejects_duplicate_issue_ids() {
    let duplicate = r#"[
      {clientId:"90000000001",name:"SYNTHETIC ALPHA LIMITED"},
      {clientId:"90000000001",name:"SYNTHETIC BETA LIMITED"}
    ]"#;
    let error = KfintechProvider::parse_issue_bundle(duplicate, "2026-08-28T18:09:00Z")
        .expect_err("duplicate provider issue ids must fail closed");
    assert_eq!(error.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn zero_all_shares_in_confirmed_record_is_not_allotted() {
    let result = parse_result(&parse_case("not_allotted")).expect("confirmed structured negative");
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotAllotted);
    assert_eq!(
        result.contract_fingerprint(),
        Some("kfin-result-data-array-v1")
    );
}

#[test]
fn confirmed_positive_record_is_allotted() {
    let result = parse_result(&parse_case("allotted")).expect("confirmed structured allotment");
    assert_eq!(result.status(), NormalizedAllotmentStatus::Allotted);
    assert_eq!(result.allotted_shares(), Some(35));
    assert_eq!(result.allotted_lots(), None);
    assert_eq!(result.provider_reference(), None);
    assert_eq!(
        result.provenance(),
        ProviderResultProvenance::ConfirmedProviderResponse
    );
}

#[test]
fn null_share_count_is_pending_not_allotted() {
    let result =
        parse_result(&parse_case("pending")).expect("pending is a typed operational state");
    assert_eq!(result.status(), NormalizedAllotmentStatus::Pending);
    assert_eq!(result.allotted_shares(), None);
    assert_eq!(
        result.provenance(),
        ProviderResultProvenance::ConfirmedProviderResponse
    );
}

#[test]
fn unrecognized_wrapper_fails_closed() {
    let err = parse_result(&parse_case("unknown")).expect_err("unknown wrapper must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn plain_text_negative_is_never_not_allotted() {
    let err = parse_result(&parse_case("malformed_zero_text"))
        .expect_err("free text cannot prove a negative result");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn ambiguous_multi_record_fails_closed() {
    let err =
        parse_result(&parse_case("duplicate")).expect_err("multiple result records are ambiguous");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn drifted_wrapper_fails_closed() {
    let err =
        parse_result(&parse_case("changed_wrapper")).expect_err("drifted wrapper must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn non_json_body_fails_closed() {
    let err = parse_result("<html>maintenance</html>").expect_err("non-json must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn empty_body_fails_closed() {
    let err = parse_result("").expect_err("empty body must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}
