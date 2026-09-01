use std::fs;
use std::path::{Path, PathBuf};

use sanket_desktop_lib::service::{
    AddFriendRequest, Application, CheckIpoInput, CheckRequest, OnboardMemberRequest,
    SubmitIpoInput, SubmitRequest,
};
use sanket_intelligence_vault::{InvestmentDecisionRequest, PlannedIpo};

fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(&path).unwrap() {
                pending.push(entry.unwrap().path());
            }
        } else if path.is_file() {
            files.push(path);
        }
    }
    files
}

fn assert_bytes_exclude(path: &Path, forbidden: &[&str]) {
    let bytes = fs::read(path).unwrap();
    for value in forbidden {
        assert!(
            !bytes
                .windows(value.len())
                .any(|window| window == value.as_bytes()),
            "{} must not contain a private identity value",
            path.display()
        );
    }
}

#[test]
fn service_never_persists_or_emits_plaintext_identity() {
    let temp = tempfile::tempdir().unwrap();
    let vault = temp.path().join("vault");
    let index_path = temp.path().join("projection.sqlite3");
    let app = Application::new(
        "security-device".to_owned(),
        vault.clone(),
        index_path.clone(),
    )
    .unwrap();

    let owner_pan = "ABCDE1234F";
    let friend_pan = "FGHIJ5678K";
    let owner_upi = "owner@okbank";
    let friend_upi = "friend@okbank";
    let forbidden = [owner_pan, friend_pan, owner_upi, friend_upi];

    let owner = app
        .onboard_member(OnboardMemberRequest {
            member_id: "member-security".to_owned(),
            display_name: "Security Owner".to_owned(),
            email: "owner@example.invalid".to_owned(),
            role: "OWNER".to_owned(),
            primary_account_label: Some("Primary".to_owned()),
            broker: Some("Broker".to_owned()),
            upi_id: owner_upi.to_owned(),
            pan: owner_pan.to_owned(),
            consented: true,
        })
        .unwrap();
    assert_eq!(owner.masked_pan, "ABCDE****F");

    let friend = app
        .add_friend(AddFriendRequest {
            friend_id: "friend-security".to_owned(),
            owner_member_id: owner.member_id.clone(),
            name: "Security Friend".to_owned(),
            upi_id: friend_upi.to_owned(),
            pan: friend_pan.to_owned(),
            broker: Some("Broker".to_owned()),
            share_eligible: true,
            share_basis_points: Some(1000),
        })
        .unwrap();
    assert_eq!(friend.masked_pan, "FGHIJ****K");

    let safe_request = InvestmentDecisionRequest::with_account_count(
        "session-security",
        1_000_000,
        vec![PlannedIpo {
            typed_name: "Example IPO".to_owned(),
            planned_amount_per_account_paise: 200_000,
        }],
        "dev-ranking-v001",
        2,
    );
    safe_request.assert_safe().unwrap();
    let ai_json = serde_json::to_string(&safe_request).unwrap();
    for value in forbidden {
        assert!(!ai_json.contains(value));
    }
    for key in [
        "pan",
        "upi",
        "email",
        "member_name",
        "friend_name",
        "proof",
        "vault_path",
    ] {
        assert!(
            !ai_json.contains(key),
            "AI request must not expose key {key}"
        );
    }

    let recommendation = app
        .check(CheckRequest {
            session_id: "session-security".to_owned(),
            declared_capital_paise: 1_000_000,
            account_ids: vec![owner.member_id.clone(), friend.friend_id.clone()],
            ipos: vec![CheckIpoInput {
                name: "Example IPO".to_owned(),
                amount_paise: 200_000,
            }],
        })
        .unwrap();
    assert!(recommendation.label.contains("NOT INVESTMENT ADVICE"));

    app.submit(SubmitRequest {
        session_id: "session-security".to_owned(),
        actor_member_id: owner.member_id,
        declared_capital_paise: 1_000_000,
        recommendation_id: None,
        ipos: vec![SubmitIpoInput {
            name: "Example IPO".to_owned(),
            amount_paise: 200_000,
            account_ids: vec!["member-security".to_owned(), "friend-security".to_owned()],
            registrar_id: "kfintech".into(),
            expected_allotment_date: None,
            metadata_snapshot: None,
            confirm_metadata_changes: false,
        }],
    })
    .unwrap();

    let mut persisted_files = files_under(&vault);
    persisted_files.push(index_path);
    assert!(
        persisted_files
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "enc"))
    );
    assert!(
        persisted_files
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "json"))
    );
    assert!(
        persisted_files
            .iter()
            .all(|path| path.extension().is_none_or(|ext| ext != "log")),
        "the service must not create log files containing request payloads"
    );
    for path in &persisted_files {
        assert_bytes_exclude(path, &forbidden);
    }

    for private_value in [owner_pan, owner_upi] {
        let error = app
            .check(CheckRequest {
                session_id: "session-rejected".to_owned(),
                declared_capital_paise: 1_000_000,
                account_ids: vec!["member-security".to_owned()],
                ipos: vec![CheckIpoInput {
                    name: private_value.to_owned(),
                    amount_paise: 200_000,
                }],
            })
            .unwrap_err();
        let rendered = format!("{error:?} {error}");
        assert!(
            !rendered.contains(private_value),
            "errors must redact rejected private values"
        );
    }
}
