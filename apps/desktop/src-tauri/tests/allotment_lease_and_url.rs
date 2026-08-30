//! Gate 4F conditions C & D proof tests.
//!
//! C: `execute_allotment_check` must FAIL CLOSED when the persisted job lacks
//!    an official status URL — never default to another registrar's portal.
//! D: the allotment lease enforces single-owner execution at the boundary:
//!    concurrent second worker refused, stale lease recoverable, finalized or
//!    cancelled jobs never re-executed.

use std::fs;
use std::path::PathBuf;

use sanket_desktop_lib::service::{
    Application, OnboardMemberRequest, StartAllotmentRequest, SubmitIpoInput, SubmitRequest,
};
use uuid::Uuid;

fn tmp() -> PathBuf {
    let p = std::env::temp_dir().join(format!("sanket-lease-{}", Uuid::now_v7()));
    fs::create_dir_all(&p).unwrap();
    p
}

/// Minimal harness: onboard owner, submit one kfintech IPO, enqueue a job.
fn queued_job() -> (Application, PathBuf, String) {
    let root = tmp();
    let vault = root.join("vault");
    let index = root.join("index.sqlite3");
    let member_id = format!("m-{}", Uuid::now_v7());
    let app = Application::new("dev-device-lease".into(), vault, index.clone()).unwrap();
    app.onboard_member(OnboardMemberRequest {
        member_id: member_id.clone(),
        display_name: "Owner".into(),
        email: "o@example.com".into(),
        role: "OWNER".into(),
        primary_account_label: Some("Primary".into()),
        broker: None,
        upi_id: "owner@upi".into(),
        pan: "ABCDE1234A".into(), // fixture → ALLOTTED
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
            name: "Lease Proof IPO".into(),
            amount_paise: 50_000,
            account_ids: vec![member_id.clone()],
            registrar_id: "kfintech".into(),
            expected_allotment_date: None,
        }],
    })
    .unwrap();
    let candidate = app.list_allotment_candidates().unwrap().remove(0);
    let queued = app
        .enqueue_allotment_check(StartAllotmentRequest {
            application_id: candidate.application_id,
            session_id: candidate.session_id,
            ipo_name: candidate.ipo_name,
            actor_member_id: member_id,
            registrar_id: Some(candidate.registrar_id),
        })
        .unwrap();
    (app, index, queued.job_id)
}

/// Open the raw SQLite index for adversarial row surgery.
fn open_raw(index: &PathBuf) -> rusqlite::Connection {
    // Note: the Application keeps its own connection open; SQLite still
    // permits this second writer between the service's transactions.
    rusqlite::Connection::open(index).unwrap()
}

// ============================================================
// Condition C — missing/invalid URL fails closed
// ============================================================

#[test]
fn missing_official_status_url_fails_closed() {
    let (app, index, job_id) = queued_job();

    // Adversarial corruption: strip the persisted URL the registry resolved.
    let conn = open_raw(&index);
    let changed = conn
        .execute("UPDATE applications SET official_status_url=NULL", [])
        .unwrap();
    assert_eq!(changed, 1);
    drop(conn);

    // The job row itself carries the URL; corrupt it too — that is the row
    // execute_allotment_check actually reads.
    let conn = open_raw(&index);
    conn.execute("UPDATE allotment_jobs SET official_status_url=NULL", [])
        .unwrap();
    drop(conn);

    let err = app.run_allotment_job_once(&job_id);
    assert!(
        err.is_err(),
        "must fail closed when the persisted job has no official status URL"
    );
    let msg = format!("{}", err.unwrap_err());
    assert!(
        msg.contains("no official status URL"),
        "error must name the missing URL, got: {msg}"
    );
    // Fail closed also means: no cross-registrar default leaked into the report.
    let report = app.get_allotment_report(&job_id).unwrap();
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("ipostatus.kfintech.com"),
        "report must not carry a defaulted registrar URL"
    );
}

#[test]
fn empty_official_status_url_fails_closed() {
    let (app, index, job_id) = queued_job();
    let conn = open_raw(&index);
    conn.execute("UPDATE allotment_jobs SET official_status_url=''", [])
        .unwrap();
    drop(conn);
    assert!(app.run_allotment_job_once(&job_id).is_err());
}

fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn set_lease(index: &PathBuf, job_id: &str, device: &str, token: &str, expires_at: i64) {
    let conn = open_raw(index);
    let changed = conn
        .execute(
            "UPDATE allotment_jobs SET lease_owner_device_id=?2,
                    lease_token=?3, lease_expires_at=?4,
                    status='RUNNING', cancel_requested=0
             WHERE id=?1",
            rusqlite::params![job_id, device, token, expires_at],
        )
        .unwrap();
    assert_eq!(changed, 1);
}

// ============================================================
// Condition D — JobLease enforcement at the execution boundary
// ============================================================

#[test]
fn same_job_two_workers_only_one_execution_owner() {
    let (app, index, job_id) = queued_job();

    // Worker A (another device) already holds a LIVE lease.
    set_lease(
        &index,
        &job_id,
        "device-A",
        "token-A",
        (epoch_now() + 120) as i64,
    );

    // Worker B (this app, device "dev-device-lease") calls the real boundary:
    // it must be REFUSED — Ok(false), no execution.
    assert!(
        !app.run_allotment_job_once(&job_id).unwrap(),
        "second worker must not execute while another device holds a live lease"
    );

    // And the refusal must not have stolen or cleared worker A's lease.
    let conn = open_raw(&index);
    let (owner, token): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT lease_owner_device_id, lease_token FROM allotment_jobs WHERE id=?1",
            rusqlite::params![job_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    drop(conn);
    assert_eq!(owner.as_deref(), Some("device-A"));
    assert_eq!(token.as_deref(), Some("token-A"));

    // The report must show no execution happened (still CREATED, no results).
    let report = app.get_allotment_report(&job_id).unwrap();
    assert_ne!(report.status, "COMPLETE");
}

#[test]
fn stale_lease_allows_safe_recovery() {
    let (app, index, job_id) = queued_job();
    // A stale lease from a crashed run: expired 60s ago (real epoch).
    set_lease(
        &index,
        &job_id,
        "device-DEAD",
        "token-DEAD",
        (epoch_now() - 60) as i64,
    );

    // Recovery: a new worker takes over and the job still completes.
    assert!(app.run_allotment_job_once(&job_id).unwrap());
    let report = app.get_allotment_report(&job_id).unwrap();
    assert_eq!(report.status, "COMPLETE", "stale lease must be recoverable");
    assert_eq!(report.accounts[0].status, "ALLOTTED"); // fixture ALLOTTED
}

#[test]
fn finalized_job_never_reexecutes() {
    let (app, _index, job_id) = queued_job();
    assert!(app.run_allotment_job_once(&job_id).unwrap());
    let first = app.get_allotment_report(&job_id).unwrap();
    assert_eq!(first.status, "COMPLETE");

    // After completion the boundary must refuse re-execution (Ok(false)).
    assert!(
        !app.run_allotment_job_once(&job_id).unwrap(),
        "COMPLETE jobs must never re-execute"
    );
    let second = app.get_allotment_report(&job_id).unwrap();
    assert_eq!(second.status, "COMPLETE");
    // Idempotence: same terminal account result, no duplicate execution rows.
    assert_eq!(second.accounts[0].status, first.accounts[0].status);
}

#[test]
fn cancelled_job_is_never_leased() {
    let (app, _index, job_id) = queued_job();
    assert!(app.cancel_allotment_job(&job_id).unwrap());
    // Cancelled job: boundary returns Ok(false) — no execution, no error loop.
    assert!(!app.run_allotment_job_once(&job_id).unwrap());
    let report = app.get_allotment_report(&job_id).unwrap();
    assert_eq!(report.status, "CANCELLED");
}
