use sanket_allotment::{
    AllotmentLookupContext, AllotmentProvider, IssueDiscoveryMode, LookupKeyKind,
    NormalizedAllotmentStatus, ProviderHealth, ProviderResultProvenance, RegistrarIssue,
    SanitizedFixtureProvenance, SessionRequirement,
};
use sanket_identity_security::Pan;

fn bigshare_provider() -> sanket_allotment::BigshareProvider {
    sanket_allotment::BigshareProvider::offline_for_tests()
}

fn lookup_context() -> AllotmentLookupContext {
    AllotmentLookupContext {
        job_id: "job-1".into(),
        attempt_id: "attempt-1".into(),
        account_id: "account-1".into(),
        issue: RegistrarIssue {
            registrar_id: "bigshare".into(),
            registrar_name: "Bigshare Services".into(),
            official_status_url: Some("https://ipo.bigshareonline.com/ipo_status.html".into()),
            issue_code: Some("9001".into()),
            ipo_name: "SYNTHETIC ALPHA LIMITED".into(),
        },
    }
}

fn parse_case(name: &str) -> String {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bigshare/cases.json")).unwrap();
    serde_json::to_string(&cases[name]).unwrap()
}

fn parse_result(
    body: &str,
) -> Result<sanket_allotment::ProviderAllotmentResult, sanket_allotment::ProviderError> {
    sanket_allotment::BigshareProvider::parse_result_body(
        body,
        true,
        "2026-08-28T17:48:00Z",
        "bigshare-result-d-status-v1",
    )
}

// ---------------------------------------------------------------
// Fixture provenance + issue discovery
// ---------------------------------------------------------------

#[test]
fn fixtures_are_verified_by_provenance() {
    let issues = include_str!("fixtures/bigshare/issues.html");
    let cases = include_str!("fixtures/bigshare/cases.json");
    SanitizedFixtureProvenance::from_json(include_str!(
        "fixtures/bigshare/issues.html.provenance.json"
    ))
    .expect("valid issue provenance")
    .verify_content(issues.as_bytes())
    .expect("issue fixture hash");
    SanitizedFixtureProvenance::from_json(include_str!(
        "fixtures/bigshare/cases.json.provenance.json"
    ))
    .expect("valid cases provenance")
    .verify_content(cases.as_bytes())
    .expect("cases fixture hash");
}

#[test]
fn discovers_current_ddl_company_issues() {
    let issues = sanket_allotment::BigshareProvider::parse_issue_bundle(
        include_str!("fixtures/bigshare/issues.html"),
        "2026-08-28T17:48:00Z",
    )
    .expect("recognized issue bundle");
    assert_eq!(issues.len(), 4);
    assert_eq!(issues[0].provider_issue_id, "9001");
    assert_eq!(issues[0].display_name, "SYNTHETIC ALPHA LIMITED");
    assert_eq!(
        issues[0].source_url,
        "https://ipo.bigshareonline.com/ipo_status.html"
    );
    assert_eq!(
        issues[0].structural_fingerprint,
        "bigshare-issues-ddlCompany-options-v1"
    );
}

#[test]
fn rejects_duplicate_issue_ids() {
    let duplicate = r#"<select id="ddlCompany">
      <option value="9001">SYNTHETIC ALPHA LIMITED</option>
      <option value="9001">SYNTHETIC BETA LIMITED</option>
    </select>"#;
    let error =
        sanket_allotment::BigshareProvider::parse_issue_bundle(duplicate, "2026-08-28T17:48:00Z")
            .expect_err("duplicate provider issue ids must fail closed");
    assert_eq!(error.to_status(), NormalizedAllotmentStatus::Unknown);
}

// ---------------------------------------------------------------
// Provider identity / capabilities / health
// ---------------------------------------------------------------

#[test]
fn offline_provider_identity_and_capabilities() {
    let p = bigshare_provider();
    assert_eq!(p.provider_id(), "bigshare-live");
    let caps = p.capabilities();
    assert_eq!(caps.issue_discovery, IssueDiscoveryMode::PublicHttp);
    assert!(caps.supports(LookupKeyKind::Pan));
    assert!(caps.supports(LookupKeyKind::ApplicationNumber));
    assert!(caps.supports(LookupKeyKind::DematAccount));
    assert_eq!(caps.session, SessionRequirement::ChallengeToken);
    assert_eq!(
        caps.human_verification,
        sanket_allotment::HumanVerificationRequirement::Required
    );
    assert_eq!(
        caps.transport,
        sanket_allotment::ProviderTransportKind::Browser
    );
    assert_eq!(
        caps.background,
        sanket_allotment::BackgroundExecution::PrepareOnly
    );
}

#[test]
fn health_states() {
    // CAPTCHA is mandatory for every new search: Bigshare is always in the
    // human-verification-required health state.
    assert_eq!(
        bigshare_provider().health(),
        ProviderHealth::HumanVerificationRequired
    );
    assert_eq!(
        sanket_allotment::BigshareProvider::new().health(),
        ProviderHealth::HumanVerificationRequired
    );
}

#[test]
fn supports_bigshare_registrar_only() {
    let p = bigshare_provider();
    let bigshare = |registrar_id: &str| RegistrarIssue {
        registrar_id: registrar_id.into(),
        registrar_name: "X".into(),
        official_status_url: None,
        issue_code: None,
        ipo_name: "X".into(),
    };
    assert!(p.supports(&bigshare("bigshare")));
    assert!(p.supports(&bigshare("bigshare-online")));
    assert!(!p.supports(&bigshare("kfintech")));
    assert!(!p.supports(&bigshare("mufg")));
}

// ---------------------------------------------------------------
// Unattended check always fails closed to NEEDS_HUMAN_VERIFICATION
// ---------------------------------------------------------------

#[test]
fn unattended_check_requires_human_verification() {
    let p = bigshare_provider();
    let pan = Pan::parse("ABCDE1234A").unwrap();
    let err = p
        .check_allotment(&lookup_context(), &pan)
        .expect_err("unattended bigshare lookup must fail closed");
    assert_eq!(
        err.to_status(),
        NormalizedAllotmentStatus::NeedsHumanVerification
    );
}

// ---------------------------------------------------------------
// Structured result parsing (fixture-driven)
// ---------------------------------------------------------------

#[test]
fn ok_positive_record_is_allotted() {
    let result = parse_result(&parse_case("ok_allotted")).expect("confirmed structured allotment");
    assert_eq!(result.status(), NormalizedAllotmentStatus::Allotted);
    assert_eq!(result.allotted_shares(), Some(35));
    assert_eq!(
        result.provenance(),
        ProviderResultProvenance::ConfirmedProviderResponse
    );
}

#[test]
fn ok_zero_shares_is_not_allotted() {
    let result =
        parse_result(&parse_case("ok_not_allotted")).expect("confirmed structured negative");
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotAllotted);
    assert_eq!(
        result.contract_fingerprint(),
        Some("bigshare-result-d-status-v1")
    );
}

#[test]
fn notfound_is_not_found_never_not_allotted() {
    let result = parse_result(&parse_case("notfound")).expect("recognized no-record state");
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotFound);
    assert_eq!(
        result.provenance(),
        ProviderResultProvenance::ConfirmedProviderResponse
    );
}

#[test]
fn captcha_status_is_needs_human_verification() {
    let result = parse_result(&parse_case("captcha")).expect("captcha is a typed state");
    assert_eq!(
        result.status(),
        NormalizedAllotmentStatus::NeedsHumanVerification
    );
}

#[test]
fn ratelimit_status_is_rate_limited() {
    let result = parse_result(&parse_case("ratelimit")).expect("rate limit is a typed state");
    assert_eq!(result.status(), NormalizedAllotmentStatus::RateLimited);
}

#[test]
fn warming_status_is_retryable() {
    let result = parse_result(&parse_case("warming")).expect("warming is a typed state");
    assert_eq!(result.status(), NormalizedAllotmentStatus::RetryableError);
}

#[test]
fn ok_null_shares_is_pending() {
    let result = parse_result(&parse_case("ok_pending")).expect("null allotment is pending");
    assert_eq!(result.status(), NormalizedAllotmentStatus::Pending);
    assert_eq!(result.allotted_shares(), None);
}

#[test]
fn ok_text_zero_is_never_not_allotted() {
    let err = parse_result(&parse_case("ok_zero_text"))
        .expect_err("text share count cannot prove a negative");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn ok_multi_match_is_never_not_allotted() {
    let err = parse_result(&parse_case("ok_multi_match"))
        .expect_err("multiple matched records are ambiguous");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn unknown_status_fails_closed() {
    let err = parse_result(&parse_case("unknown_status"))
        .expect_err("unrecognized status must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn missing_status_fails_closed() {
    let err =
        parse_result(&parse_case("missing_status")).expect_err("missing status must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn drifted_wrapper_fails_closed() {
    let err =
        parse_result(&parse_case("changed_wrapper")).expect_err("drifted wrapper must fail closed");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn rejected_fingerprint_fails_closed() {
    let err = sanket_allotment::BigshareProvider::parse_result_body(
        &parse_case("ok_allotted"),
        true,
        "2026-08-28T17:48:00Z",
        "some-other-fingerprint",
    )
    .expect_err("unaccepted contract fingerprint must fail closed");
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

#[test]
fn parser_never_copies_member_fields() {
    // The result must not surface APPLICATION_NO/DPID/Name values.
    let result = parse_result(&parse_case("ok_allotted")).expect("structured result");
    assert_eq!(result.provider_reference(), None);
    assert_eq!(result.safe_message(), None);
}

// ---------------------------------------------------------------
// Challenge lifecycle: safe metadata, legal transitions, expiry,
// cancellation, restart semantics
// ---------------------------------------------------------------

use sanket_allotment::{
    HumanVerificationChallenge, HumanVerificationStatus, HumanVerificationType,
    ProviderContinuationReference,
};

fn challenge(status: HumanVerificationStatus) -> HumanVerificationChallenge {
    HumanVerificationChallenge::new(
        "challenge-1",
        "bigshare-live",
        "job-1",
        "attempt-1",
        "account-1",
        HumanVerificationType::Captcha,
        status,
        "server-1",
        "2026-08-29T00:00:00Z",
        Some("2026-08-29T00:05:00Z".into()),
        ProviderContinuationReference::new("continuation-1").unwrap(),
    )
    .unwrap()
}

#[test]
fn challenge_progresses_required_presented_completed() {
    let c = challenge(HumanVerificationStatus::Required);
    let presented = c
        .transition(HumanVerificationStatus::Presented)
        .expect("user opened the isolated verification surface");
    let completed = presented
        .transition(HumanVerificationStatus::Completed)
        .expect("user legitimately completed the challenge");
    assert_eq!(completed.status(), HumanVerificationStatus::Completed);
}

#[test]
fn challenge_cannot_skip_presented_or_regress() {
    assert!(
        challenge(HumanVerificationStatus::Required)
            .transition(HumanVerificationStatus::Completed)
            .is_err()
    );
    let completed = challenge(HumanVerificationStatus::Completed);
    assert!(
        completed
            .transition(HumanVerificationStatus::Required)
            .is_err()
    );
    assert!(
        completed
            .transition(HumanVerificationStatus::Presented)
            .is_err()
    );
}

#[test]
fn challenge_cancel_and_expire_from_active_states_only() {
    for active in [
        HumanVerificationStatus::Required,
        HumanVerificationStatus::Presented,
    ] {
        challenge(active)
            .transition(HumanVerificationStatus::Cancelled)
            .expect("user may cancel an active challenge");
        challenge(active)
            .transition(HumanVerificationStatus::Expired)
            .expect("an active challenge may expire");
    }
}

#[test]
fn terminal_challenge_states_are_final() {
    for terminal in [
        HumanVerificationStatus::Completed,
        HumanVerificationStatus::Expired,
        HumanVerificationStatus::Cancelled,
    ] {
        for next in [
            HumanVerificationStatus::Required,
            HumanVerificationStatus::Presented,
            HumanVerificationStatus::Completed,
            HumanVerificationStatus::Expired,
            HumanVerificationStatus::Cancelled,
        ] {
            assert!(
                challenge(terminal).transition(next).is_err(),
                "terminal {terminal:?} must not transition to {next:?}"
            );
        }
    }
}

#[test]
fn challenge_expiry_is_detected_and_resumable_window_closes() {
    let c = challenge(HumanVerificationStatus::Presented);
    assert!(!c.is_expired("2026-08-29T00:04:59Z"));
    assert!(c.is_expired("2026-08-29T00:05:00Z"));
    assert!(c.is_expired("2026-08-29T00:06:00Z"));
    // An expired challenge can no longer be completed — a fresh challenge
    // is required (restart/expiry semantics).
    let expired = c
        .transition(HumanVerificationStatus::Expired)
        .expect("expiry transition");
    assert!(
        expired
            .transition(HumanVerificationStatus::Completed)
            .is_err()
    );
}

#[test]
fn restart_requires_fresh_challenge_and_refresh_transition() {
    // Restart reconciliation: an expired challenge is terminal, and the job
    // re-enters the prepare state to create a fresh legitimate challenge.
    let expired = challenge(HumanVerificationStatus::Expired);
    assert!(
        expired
            .transition(HumanVerificationStatus::Presented)
            .is_err()
    );

    let fresh = HumanVerificationChallenge::new(
        "challenge-2",
        "bigshare-live",
        "job-1",
        "attempt-1",
        "account-1",
        HumanVerificationType::Captcha,
        HumanVerificationStatus::Required,
        "server-1",
        "2026-08-29T01:00:00Z",
        Some("2026-08-29T01:05:00Z".into()),
        ProviderContinuationReference::new("continuation-2").unwrap(),
    )
    .unwrap();
    assert_ne!(
        expired.continuation_reference(),
        fresh.continuation_reference()
    );
    assert_eq!(fresh.status(), HumanVerificationStatus::Required);
}
