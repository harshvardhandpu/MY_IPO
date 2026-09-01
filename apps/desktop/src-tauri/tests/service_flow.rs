use std::fs;

use sanket_desktop_lib::service::{
    Application, CheckIpoInput, CheckRequest, HistoricalApplicationRequest, OnboardMemberRequest,
    SubmitIpoInput, SubmitRequest, VoidSessionRequest,
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
                metadata_snapshot: None,
                confirm_metadata_changes: false,
            }],
        })
        .unwrap();
    assert_eq!(submitted.allocation_count, 1);

    let dashboard = app.dashboard().unwrap();
    assert_eq!(dashboard.submitted_session_count, 1);
    assert_eq!(dashboard.member_count, 1);
    assert_eq!(dashboard.total_planned_paise, 1_000_000);
}

#[test]
fn historical_application_records_owner_chain_mapping_without_job_or_pan() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    let index = temp.path().join("index.sqlite3");
    let app = Application::new("device-test-01".to_owned(), vault, index.clone()).unwrap();
    let pan = synthetic_pan();
    app.onboard_member(OnboardMemberRequest {
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

    let saved = app
        .record_historical_application(HistoricalApplicationRequest {
            actor_member_id: "member-1".to_owned(),
            account_id: "member-1".to_owned(),
            ipo_name: "Symbiotec Pharmalab Limited".to_owned(),
            amount_paise: 1_482_000,
            application_date: None,
            registrar_id: "mufg_intime".to_owned(),
            provider_issue_id: "11926".to_owned(),
            owner_affirmed: true,
        })
        .unwrap();

    assert_eq!(saved.source, "OWNER_HISTORICAL_ENTRY");
    assert_eq!(saved.provider_issue_id, "11926");
    let db = rusqlite::Connection::open(&index).unwrap();
    let application: (String, i64, String, Option<String>) = db
        .query_row(
            "SELECT ipo_name, planned_amount_paise, source, application_date FROM applications WHERE id=?1",
            [&saved.application_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        application,
        (
            "Symbiotec Pharmalab Limited".to_owned(),
            1_482_000,
            "OWNER_HISTORICAL_ENTRY".to_owned(),
            None,
        )
    );
    let allocation_account: String = db
        .query_row(
            "SELECT account_id FROM allocations WHERE application_id=?1",
            [&saved.application_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(allocation_account, "member-1");
    let mapping: (String, String) = db
        .query_row(
            "SELECT provider_id, provider_issue_id FROM provider_issue_mappings WHERE application_id=?1",
            [&saved.application_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(mapping, ("mufg-intime-live".to_owned(), "11926".to_owned()));
    let job_count: i64 = db
        .query_row("SELECT COUNT(*) FROM allotment_jobs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(job_count, 0);
    let attempt_count: i64 = db
        .query_row("SELECT COUNT(*) FROM allotment_attempts", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(attempt_count, 0);
    let sqlite_bytes = fs::read(index).unwrap();
    assert!(
        !sqlite_bytes
            .windows(pan.len())
            .any(|window| window == pan.as_bytes())
    );
}

#[test]
fn historical_application_rejects_unaffirmed_invalid_and_duplicate_entries() {
    let temp = tempfile::tempdir().unwrap();
    let app = Application::new(
        "device-test-01".to_owned(),
        temp.path().join("vault"),
        temp.path().join("index.sqlite3"),
    )
    .unwrap();
    app.onboard_member(OnboardMemberRequest {
        member_id: "member-1".to_owned(),
        display_name: "Owner".to_owned(),
        email: "owner@example.invalid".to_owned(),
        role: "OWNER".to_owned(),
        primary_account_label: Some("Primary".to_owned()),
        broker: None,
        upi_id: "owner@okbank".to_owned(),
        pan: synthetic_pan(),
        consented: true,
    })
    .unwrap();

    let request =
        |owner_affirmed: bool, account_id: &str, application_date: Option<&str>, issue: &str| {
            HistoricalApplicationRequest {
                actor_member_id: "member-1".to_owned(),
                account_id: account_id.to_owned(),
                ipo_name: "Symbiotec Pharmalab Limited".to_owned(),
                amount_paise: 1_482_000,
                application_date: application_date.map(str::to_owned),
                registrar_id: "mufg_intime".to_owned(),
                provider_issue_id: issue.to_owned(),
                owner_affirmed,
            }
        };
    assert!(
        app.record_historical_application(request(false, "member-1", None, "11926"))
            .is_err()
    );
    assert!(
        app.record_historical_application(request(true, "other", None, "11926"))
            .is_err()
    );
    assert!(
        app.record_historical_application(request(true, "member-1", Some("2026-02-31"), "11926"))
            .is_err()
    );
    assert!(
        app.record_historical_application(request(true, "member-1", None, "issue-11926"))
            .is_err()
    );

    app.record_historical_application(request(true, "member-1", None, "11926"))
        .unwrap();
    let duplicate = app
        .record_historical_application(request(true, "member-1", None, "11926"))
        .unwrap_err();
    assert!(duplicate.to_string().contains("already exists"));
}

#[test]
fn voided_historical_application_allows_replacement_but_active_duplicate_still_rejects() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    let index = temp.path().join("index.sqlite3");
    let app = Application::new(
        "device-test-01".to_owned(),
        vault.clone(),
        index.clone(),
    )
    .unwrap();
    app.onboard_member(OnboardMemberRequest {
        member_id: "member-1".to_owned(),
        display_name: "Owner".to_owned(),
        email: "owner@example.invalid".to_owned(),
        role: "OWNER".to_owned(),
        primary_account_label: Some("Primary".to_owned()),
        broker: None,
        upi_id: "owner@okbank".to_owned(),
        pan: synthetic_pan(),
        consented: true,
    })
    .unwrap();

    let request = || HistoricalApplicationRequest {
        actor_member_id: "member-1".to_owned(),
        account_id: "member-1".to_owned(),
        ipo_name: "Symbiotec Pharmalab Limited".to_owned(),
        amount_paise: 1_482_000,
        application_date: None,
        registrar_id: "mufg_intime".to_owned(),
        provider_issue_id: "11926".to_owned(),
        owner_affirmed: true,
    };

    let original = app.record_historical_application(request()).unwrap();
    app.void_submitted_session(VoidSessionRequest {
        session_id: original.session_id.clone(),
        actor_member_id: "member-1".to_owned(),
        reason: "replace duplicate historical entry".to_owned(),
        owner_affirmed: true,
    })
    .unwrap();

    let replacement = app.record_historical_application(request()).unwrap();
    let duplicate = app.record_historical_application(request()).unwrap_err();
    assert!(duplicate.to_string().contains("already exists"));

    let db = rusqlite::Connection::open(&index).unwrap();
    let original_status: String = db
        .query_row(
            "SELECT status FROM investment_sessions WHERE id=?1",
            [&original.session_id],
            |row| row.get(0),
        )
        .unwrap();
    let replacement_status: String = db
        .query_row(
            "SELECT status FROM investment_sessions WHERE id=?1",
            [&replacement.session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(original_status, "VOIDED");
    assert_eq!(replacement_status, "SUBMITTED");

    let active_equivalent_count: i64 = db
        .query_row(
            "SELECT COUNT(*)
             FROM applications a
             JOIN investment_sessions s ON s.id=a.session_id
             JOIN provider_issue_mappings p ON p.application_id=a.id
             WHERE s.actor_member_id=?1
               AND s.status='SUBMITTED'
               AND lower(trim(a.ipo_name))=lower(trim(?2))
               AND a.source='OWNER_HISTORICAL_ENTRY'
               AND p.provider_id='mufg-intime-live'
               AND p.provider_issue_id='11926'",
            ["member-1", "Symbiotec Pharmalab Limited"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(active_equivalent_count, 1);

    let event_contents: Vec<String> = fs::read_dir(vault.join("_events"))
        .unwrap()
        .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    assert!(event_contents.iter().any(|event| {
        event.contains(&original.application_id)
            && event.contains("OWNER_HISTORICAL_ENTRY")
    }));
    assert!(event_contents.iter().any(|event| {
        event.contains(&original.session_id)
            && event.contains("INVESTMENT_SESSION_VOIDED")
    }));
}
