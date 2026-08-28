use sanket_allotment::{
    AllotmentLookupContext, AllotmentProvider, LiveKfintechProvider, NormalizedAllotmentStatus,
    ProviderHealth, RegistrarIssue,
};
use sanket_identity_security::Pan;

#[test]
fn offline_live_never_not_allotted() {
    let p = LiveKfintechProvider::offline_for_tests();
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
    let p = LiveKfintechProvider::human_gate_for_tests();
    assert_eq!(p.health(), ProviderHealth::HumanVerificationRequired);
}
