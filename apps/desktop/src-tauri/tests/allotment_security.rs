//! Allotment check must never leave plaintext PAN in events, SQLite, or report JSON.

use std::fs;
use std::path::PathBuf;

use sanket_desktop_lib::service::{
    Application, OnboardMemberRequest, StartAllotmentRequest, SubmitIpoInput, SubmitRequest,
};
use uuid::Uuid;

fn tmp() -> PathBuf {
    let p = std::env::temp_dir().join(format!("sanket-allot-{}", Uuid::now_v7()));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn allotment_fixture_never_persists_plaintext_pan() {
    let root = tmp();
    let vault = root.join("vault");
    let index = root.join("index.sqlite3");
    let app = Application::new("dev-device-allot".into(), vault, index).unwrap();

    const FULL_PAN: &str = "ABCDE1234A"; // fixture → ALLOTTED
    let member_id = format!("m-{}", Uuid::now_v7());
    app.onboard_member(OnboardMemberRequest {
        member_id: member_id.clone(),
        display_name: "Owner".into(),
        email: "o@example.com".into(),
        role: "OWNER".into(),
        primary_account_label: Some("Primary".into()),
        broker: None,
        upi_id: "owner@upi".into(),
        pan: FULL_PAN.into(),
        consented: true,
    })
    .unwrap();

    let session_id = format!("s-{}", Uuid::now_v7());
    app.submit(SubmitRequest {
        session_id: session_id.clone(),
        actor_member_id: member_id.clone(),
        declared_capital_paise: 100_000,
        recommendation_id: None,
        ipos: vec![SubmitIpoInput {
            name: "Fixture IPO".into(),
            amount_paise: 50_000,
            account_ids: vec![member_id.clone()],
        }],
    })
    .unwrap();

    let apps = app.list_allotment_candidates().unwrap();
    assert_eq!(apps.len(), 1);
    let report = app
        .start_allotment_check(StartAllotmentRequest {
            application_id: apps[0].application_id.clone(),
            session_id: apps[0].session_id.clone(),
            ipo_name: apps[0].ipo_name.clone(),
            actor_member_id: member_id,
            registrar_id: Some("kfintech".into()),
            registrar_name: Some("KFintech".into()),
            official_status_url: None,
        })
        .unwrap();

    assert_eq!(report.accounts.len(), 1);
    assert_eq!(report.accounts[0].status, "ALLOTTED");
    assert!(!report.accounts[0].masked_pan.contains("1234"));

    let report_json = serde_json::to_string(&report).unwrap();
    assert!(
        !report_json.contains(FULL_PAN),
        "report card leaked PAN: {report_json}"
    );

    // Scan vault root for plaintext PAN bytes.
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let bytes = fs::read(&path).unwrap_or_default();
            let text = String::from_utf8_lossy(&bytes);
            assert!(
                !text.contains(FULL_PAN),
                "plaintext PAN found in {}",
                path.display()
            );
        }
    }
}
