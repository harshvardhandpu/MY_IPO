use sanket_allotment::{
    AllotmentLookupContext, AllotmentProvider, FixtureKfintechProvider, NormalizedAllotmentStatus,
    RegistrarIssue,
};
use sanket_identity_security::Pan;

fn issue() -> RegistrarIssue {
    RegistrarIssue {
        registrar_id: "kfintech".into(),
        registrar_name: "KFintech".into(),
        official_status_url: Some("https://ipostatus.kfintech.com".into()),
        issue_code: Some("TEST".into()),
        ipo_name: "Test IPO".into(),
    }
}

fn ctx() -> AllotmentLookupContext {
    AllotmentLookupContext {
        job_id: "job-1".into(),
        attempt_id: "att-1".into(),
        account_id: "acct-1".into(),
        issue: issue(),
    }
}

#[test]
fn fixture_allots_for_pan_ending_a() {
    // Synthetic format-valid PAN for fixture routing only.
    let pan = Pan::parse("ABCDE1234A").expect("pan");
    let p = FixtureKfintechProvider;
    let r = p.check_allotment(&ctx(), &pan).expect("ok");
    assert_eq!(r.status(), NormalizedAllotmentStatus::Allotted);
    let json = serde_json::to_string(&r).unwrap();
    assert!(!json.contains(pan.as_normalized()));
}

#[test]
fn fixture_unknown_path_is_error_unknown_not_not_allotted() {
    let pan = Pan::parse("ABCDE1234G").expect("pan");
    let p = FixtureKfintechProvider;
    let err = p.check_allotment(&ctx(), &pan).expect_err("unknown");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}
