//! Allotment check must never leave plaintext PAN in events, SQLite, or report JSON.

use std::fs;
use std::path::PathBuf;

use sanket_desktop_lib::service::{
    Application, ManualAllotmentRequest, OnboardMemberRequest, StartAllotmentRequest,
    SubmitIpoInput, SubmitRequest,
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
            registrar_id: "kfintech".into(),
            expected_allotment_date: None,
            metadata_snapshot: None,
            confirm_metadata_changes: false,
        }],
    })
    .unwrap();

    let apps = app.list_allotment_candidates().unwrap();
    assert_eq!(apps.len(), 1);
    let queued = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: apps[0].application_id.clone(),
            session_id: apps[0].session_id.clone(),
            ipo_name: apps[0].ipo_name.clone(),
            actor_member_id: member_id.clone(),
            registrar_id: Some("kfintech".into()),
        })
        .unwrap();

    assert_eq!(queued.status, "WAITING_FOR_AUTHORIZATION");
    assert_eq!(queued.accounts.len(), 1);
    assert_eq!(queued.accounts[0].status, "PENDING");
    assert!(!queued.accounts[0].masked_pan.contains("1234"));
    assert!(app.run_allotment_job_once(&queued.job_id).unwrap());
    let report = app.get_allotment_report(&queued.job_id).unwrap();

    assert_eq!(report.accounts.len(), 1);
    assert_eq!(report.accounts[0].status, "UNRESOLVED");
    assert_eq!(report.accounts[0].resolution_state, "UNRESOLVED");
    assert_eq!(report.accounts[0].provenance, "UNCONFIRMED_ATTEMPT");
    assert_eq!(report.final_count, 0);
    assert!(!report.accounts[0].masked_pan.contains("1234"));

    let manual_err = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: queued.job_id.clone(),
            account_id: report.accounts[0].account_id.clone(),
            actor_member_id: member_id.clone(),
            result: None,
            allotted_lots: None,
            allotted_shares: None,
            explicit_not_allotted: false,
            official_source: None,
            note: Some(FULL_PAN.into()),
        })
        .expect_err("manual provenance must reject a PAN before persisting");
    assert!(manual_err.to_string().contains("must not contain a PAN"));

    let wrong_actor_err = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: queued.job_id.clone(),
            account_id: report.accounts[0].account_id.clone(),
            actor_member_id: "different-member".into(),
            result: None,
            allotted_lots: None,
            allotted_shares: None,
            explicit_not_allotted: false,
            official_source: None,
            note: None,
        })
        .expect_err("manual provenance must reject the wrong actor before persisting");
    assert!(wrong_actor_err.to_string().contains("does not own"));

    let manual_negative = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: queued.job_id.clone(),
            account_id: report.accounts[0].account_id.clone(),
            actor_member_id: member_id.clone(),
            result: Some("NOT_ALLOTTED".into()),
            allotted_lots: None,
            allotted_shares: None,
            explicit_not_allotted: true,
            official_source: Some("https://ipostatus.kfintech.com/status".into()),
            note: None,
        })
        .expect("authorized manual result should persist");
    assert_eq!(manual_negative.status, "NOT_ALLOTTED");
    assert_eq!(manual_negative.resolution_state, "MANUAL_CONFIRMED");

    let unsupported_err = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: apps[0].application_id.clone(),
            session_id: apps[0].session_id.clone(),
            ipo_name: apps[0].ipo_name.clone(),
            actor_member_id: "member".into(),
            registrar_id: Some("unknown-registrar".into()),
        })
        .expect_err("unknown registrars must fail closed without provider guessing");
    assert!(
        unsupported_err
            .to_string()
            .contains("unsupported registrar")
    );

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

#[test]
fn bigshare_job_recovers_cancels_and_preserves_manual_provenance_without_pan_access() {
    let root = tmp();
    let vault = root.join("vault");
    let index = root.join("index.sqlite3");
    let member_id = format!("m-{}", Uuid::now_v7());
    let session_id = format!("s-{}", Uuid::now_v7());
    let app = Application::new("dev-device-allot".into(), vault.clone(), index.clone()).unwrap();
    app.onboard_member(OnboardMemberRequest {
        member_id: member_id.clone(),
        display_name: "Owner".into(),
        email: "o@example.com".into(),
        role: "OWNER".into(),
        primary_account_label: Some("Primary".into()),
        broker: None,
        upi_id: "owner@upi".into(),
        pan: "ABCDE1234A".into(),
        consented: true,
    })
    .unwrap();
    app.submit(SubmitRequest {
        session_id: session_id.clone(),
        actor_member_id: member_id.clone(),
        declared_capital_paise: 100_000,
        recommendation_id: None,
        ipos: vec![SubmitIpoInput {
            name: "Bigshare Fixture IPO".into(),
            amount_paise: 50_000,
            account_ids: vec![member_id.clone()],
            registrar_id: "bigshare".into(),
            expected_allotment_date: Some("2026-08-30".into()),
            metadata_snapshot: None,
            confirm_metadata_changes: false,
        }],
    })
    .unwrap();
    let candidate = app.list_allotment_candidates().unwrap().remove(0);
    assert_eq!(candidate.provider_id, "bigshare-live");
    assert_eq!(candidate.provider_health, "HUMAN_VERIFICATION_REQUIRED");
    let queued = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: candidate.application_id.clone(),
            session_id: candidate.session_id.clone(),
            ipo_name: candidate.ipo_name.clone(),
            actor_member_id: member_id.clone(),
            registrar_id: Some(candidate.registrar_id.clone()),
        })
        .unwrap();
    drop(app);

    let app = Application::new("dev-device-allot".into(), vault, index).unwrap();
    assert_eq!(
        app.resumable_allotment_job_ids().unwrap(),
        vec![queued.job_id.clone()]
    );
    assert!(app.cancel_allotment_job(&queued.job_id).unwrap());
    let cancelled = app.get_allotment_report(&queued.job_id).unwrap();
    assert_eq!(cancelled.status, "CANCELLED");
    assert_eq!(cancelled.accounts[0].status, "CANCELLED");

    let retried = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: candidate.application_id,
            session_id: candidate.session_id,
            ipo_name: candidate.ipo_name,
            actor_member_id: member_id.clone(),
            registrar_id: Some(candidate.registrar_id),
        })
        .unwrap();
    assert!(app.run_allotment_job_once(&retried.job_id).unwrap());
    let verification = app.get_allotment_report(&retried.job_id).unwrap();
    assert_eq!(verification.status, "INTERACTION_REQUIRED");
    assert_eq!(
        verification.accounts[0].human_verification_state.as_deref(),
        Some("REQUIRED")
    );
    assert_eq!(
        verification.accounts[0].provenance,
        "PROVIDER_OPERATIONAL_STATE"
    );
    assert!(
        !serde_json::to_string(&verification)
            .unwrap()
            .contains("ABCDE1234A")
    );

    let manual = app
        .record_manual_allotment_result(ManualAllotmentRequest {
            job_id: retried.job_id,
            account_id: member_id.clone(),
            actor_member_id: member_id,
            result: Some("NOT_ALLOTTED".into()),
            allotted_lots: None,
            allotted_shares: None,
            explicit_not_allotted: true,
            official_source: Some("https://ipo.bigshareonline.com/ipo_status.html".into()),
            note: None,
        })
        .unwrap();
    assert_eq!(manual.source, "MANUAL");
    assert_eq!(manual.provenance, "OFFICIAL_MANUAL_SOURCE");
    let _ = fs::remove_dir_all(root);
}
