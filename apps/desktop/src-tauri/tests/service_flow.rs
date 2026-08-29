use std::fs;

use sanket_desktop_lib::service::{
    Application, CheckIpoInput, CheckRequest, OnboardMemberRequest, SubmitIpoInput, SubmitRequest,
};

fn synthetic_pan() -> String {
    ["ABCDE", "1234", "F"].concat()
}

#[test]
fn onboarding_check_submit_flow_keeps_plaintext_identity_out_of_projection() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    let index = temp.path().join("index.sqlite3");
    let app = Application::new("device-test-01".to_owned(), vault.clone(), index.clone()).unwrap();

    let pan = synthetic_pan();
    let onboarded = app
        .onboard_member(OnboardMemberRequest {
            member_id: "member-1".to_owned(),
            display_name: "Owner".to_owned(),
            email: "owner@example.invalid".to_owned(),
            role: "OWNER".to_owned(),
            primary_account_label: Some("Primary".to_owned()),
            broker: Some("Broker".to_owned()),
            upi_id: "owner@okbank".to_owned(),
            pan: pan.clone(),
            consented: true,
        })
        .unwrap();

    assert_eq!(onboarded.masked_pan, "ABCDE****F");
    let members = app.list_members().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].masked_pan, "ABCDE****F");

    let secure_path = vault.join("_secure_identity/member-1.enc");
    let secure_bytes = fs::read(&secure_path).unwrap();
    assert!(
        !secure_bytes
            .windows(pan.len())
            .any(|window| window == pan.as_bytes())
    );
    let sqlite_bytes = fs::read(&index).unwrap();
    assert!(
        !sqlite_bytes
            .windows(pan.len())
            .any(|window| window == pan.as_bytes())
    );

    let checked = app
        .check(CheckRequest {
            session_id: "session-1".to_owned(),
            declared_capital_paise: 1_000_000,
            account_ids: vec!["member-1".to_owned()],
            ipos: vec![CheckIpoInput {
                name: "Example IPO".to_owned(),
                amount_paise: 200_000,
            }],
        })
        .unwrap();
    assert_eq!(checked.algorithm_version, "dev-ranking-v001");
    assert!(checked.label.contains("NOT INVESTMENT ADVICE"));
    assert_eq!(checked.ipos.len(), 1);

    let submitted = app
        .submit(SubmitRequest {
            session_id: "session-1".to_owned(),
            actor_member_id: "member-1".to_owned(),
            declared_capital_paise: 1_000_000,
            recommendation_id: None,
            ipos: vec![SubmitIpoInput {
                name: "Example IPO".to_owned(),
                amount_paise: 200_000,
                account_ids: vec!["member-1".to_owned()],
                registrar_id: "kfintech".into(),
                expected_allotment_date: None,
            }],
        })
        .unwrap();
    assert_eq!(submitted.allocation_count, 1);

    let dashboard = app.dashboard().unwrap();
    assert_eq!(dashboard.submitted_session_count, 1);
    assert_eq!(dashboard.member_count, 1);
    assert_eq!(dashboard.total_planned_paise, 1_000_000);
}
