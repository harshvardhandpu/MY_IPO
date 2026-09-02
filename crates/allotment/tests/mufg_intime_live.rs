use sanket_allotment::MufgIntimeProvider;
use sanket_allotment::{
    AllotmentLookupContext, AllotmentProvider, IssueDiscoveryMode, LookupKeyKind,
    NormalizedAllotmentStatus, ProviderHealth, ProviderResultProvenance, RegistrarIssue,
    SanitizedFixtureProvenance, SessionRequirement,
};
use sanket_identity_security::Pan;

fn mufg_provider() -> sanket_allotment::MufgIntimeProvider {
    sanket_allotment::MufgIntimeProvider::offline_for_tests()
}

fn lookup_context() -> AllotmentLookupContext {
    AllotmentLookupContext {
        job_id: "job-1".into(),
        attempt_id: "attempt-1".into(),
        account_id: "account-1".into(),
        issue: RegistrarIssue {
            registrar_id: "mufg intime".into(),
            registrar_name: "MUFG Intime India".into(),
            official_status_url: Some(
                "https://in.mpms.mufg.com/Initial_Offer/public-issues.html".into(),
            ),
            issue_code: Some("11926".into()),
            ipo_name: "SYNTHETIC ALPHA LIMITED".into(),
        },
    }
}

fn parse_case(name: &str) -> String {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/cases.json")).unwrap();
    cases[name]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| serde_json::to_string(&cases[name]).unwrap())
}

fn captcha_case(name: &str) -> String {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/captcha_current.json")).unwrap();
    cases[name].as_str().unwrap().to_owned()
}

fn parse_result(
    body: &str,
) -> Result<sanket_allotment::ProviderAllotmentResult, sanket_allotment::ProviderError> {
    sanket_allotment::MufgIntimeProvider::parse_result_body(
        body,
        true,
        "2026-08-28T18:09:00Z",
        "mufg-result-d-xml-table-v1",
    )
}

// ---------------------------------------------------------------
// Fixture provenance + issue discovery
// ---------------------------------------------------------------

#[test]
fn fixtures_are_verified_by_provenance() {
    let issues = include_str!("fixtures/mufg/issues.json");
    let cases = include_str!("fixtures/mufg/cases.json");
    SanitizedFixtureProvenance::from_json(include_str!(
        "fixtures/mufg/issues.json.provenance.json"
    ))
    .expect("valid issue provenance")
    .verify_content(issues.as_bytes())
    .expect("issue fixture hash");
    SanitizedFixtureProvenance::from_json(include_str!("fixtures/mufg/cases.json.provenance.json"))
        .expect("valid case provenance")
        .verify_content(cases.as_bytes())
        .expect("case fixture hash");
    SanitizedFixtureProvenance::from_json(include_str!(
        "fixtures/mufg/captcha_current.json.provenance.json"
    ))
    .expect("valid current captcha provenance")
    .verify_content(include_bytes!("fixtures/mufg/captcha_current.json"))
    .expect("current captcha fixture hash");
}

#[test]
fn discovers_json_wrapped_xml_issue_records() {
    let issues = include_str!("fixtures/mufg/issues.json");
    let cases: serde_json::Value = serde_json::from_str(issues).unwrap();
    let body = serde_json::to_string(&cases["discovery"]).unwrap();
    let parsed =
        sanket_allotment::MufgIntimeProvider::parse_issue_bundle(&body, "2026-08-28T18:09:00Z")
            .expect("parsed discovery records");
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].provider_issue_id, "11926");
    assert_eq!(parsed[0].display_name, "SYNTHETIC ALPHA LIMITED");
    assert_eq!(parsed[3].provider_issue_id, "11923");
}

#[test]
fn rejects_placeholder_invalid_or_duplicate_issue_ids() {
    let issues = include_str!("fixtures/mufg/issues.json");
    let cases: serde_json::Value = serde_json::from_str(issues).unwrap();
    let placeholder = serde_json::to_string(&cases["discovery_placeholder"]).unwrap();
    assert!(
        sanket_allotment::MufgIntimeProvider::parse_issue_bundle(
            &placeholder,
            "2026-08-28T18:09:00Z"
        )
        .is_err()
    );
    let duplicate = serde_json::to_string(&cases["discovery_duplicate"]).unwrap();
    assert!(
        sanket_allotment::MufgIntimeProvider::parse_issue_bundle(
            &duplicate,
            "2026-08-28T18:09:00Z"
        )
        .is_err()
    );
    let drifted = serde_json::to_string(&cases["discovery_drifted"]).unwrap();
    assert!(
        sanket_allotment::MufgIntimeProvider::parse_issue_bundle(
            &drifted,
            "2026-08-28T18:09:00ZZ".trim_end_matches('Z')
        )
        .is_err()
    );
}

#[test]
fn rejects_trailing_issue_without_company_name() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/issues.json")).unwrap();
    let incomplete = serde_json::to_string(&cases["discovery_trailing_incomplete"]).unwrap();
    assert!(MufgIntimeProvider::parse_issue_bundle(&incomplete, "2026-08-28T18:09:00Z").is_err());
}

#[test]
fn provider_contract_capabilities_match_design() {
    let provider = mufg_provider();
    let capabilities = provider.capabilities();
    assert_eq!(capabilities.issue_discovery, IssueDiscoveryMode::PublicHttp);
    assert!(capabilities.lookup_keys.contains(&LookupKeyKind::Pan));
    assert!(
        capabilities
            .lookup_keys
            .contains(&LookupKeyKind::ApplicationNumber)
    );
    assert!(
        capabilities
            .lookup_keys
            .contains(&LookupKeyKind::DematAccount)
    );
    assert!(
        capabilities
            .lookup_keys
            .contains(&LookupKeyKind::BankAccountAndIfsc)
    );
    assert_eq!(
        capabilities.session,
        SessionRequirement::CookieAndRequestToken
    );
    assert_eq!(
        capabilities.transport,
        sanket_allotment::ProviderTransportKind::Hybrid
    );
    assert_eq!(
        capabilities.background,
        sanket_allotment::BackgroundExecution::PrepareOnly
    );
}

#[test]
fn registry_resolves_mufg_ids_and_aliases() {
    assert_eq!(
        sanket_allotment::ProviderRegistry::resolve("mufg intime"),
        Some(sanket_allotment::ProviderId::MufgIntimeLive)
    );
    assert_eq!(
        sanket_allotment::ProviderRegistry::resolve("link intime"),
        Some(sanket_allotment::ProviderId::MufgIntimeLive)
    );
}

// ---------------------------------------------------------------
// Session/token representation
// ---------------------------------------------------------------

#[test]
fn keeps_cookie_and_request_token_ephemeral() {
    // Debug must never leak the cookie or token: static placeholder only.
    use sanket_allotment::MufgEphemeralSession;
    let session = MufgEphemeralSession::new(
        "session-1",
        "cookie-synthetic",
        "token-synthetic",
        "2026-08-28T18:09:00Z",
        Some("2026-08-28T18:14:00Z".into()),
    )
    .expect("session");
    let debug = format!("{session:?}");
    assert!(!debug.contains("token-synthetic"));
    assert!(!debug.contains("cookie-synthetic"));
}

#[test]
fn session_expiry_is_fail_safe() {
    use sanket_allotment::MufgEphemeralSession;
    let session = MufgEphemeralSession::new(
        "session-1",
        "cookie-synthetic",
        "token-synthetic",
        "2026-08-28T18:09:00Z",
        Some("2026-08-28T18:14:00Z".into()),
    )
    .expect("session");
    assert!(!session.is_expired("2026-08-28T18:13:59Z"));
    assert!(session.is_expired("2026-08-28T18:14:00Z"));
    // No expiry => never expires by time alone.
    let open = MufgEphemeralSession::new(
        "session-2",
        "cookie-synthetic",
        "token-synthetic",
        "2026-08-28T18:09:00Z",
        None,
    )
    .expect("session");
    assert!(!open.is_expired("2099-01-01T00:00:00Z"));
}

// ---------------------------------------------------------------
// CAPTCHA detection from page markup
// ---------------------------------------------------------------

#[test]
fn captcha_detection_absent_dormant_required() {
    use sanket_allotment::MufgCaptchaState;
    let absent =
        MufgIntimeProvider::captcha_state("<div id=\"panel\"></div><div>No challenge</div>");
    assert_eq!(absent.expect("state"), MufgCaptchaState::Absent);
    let dormant = MufgIntimeProvider::captcha_state(
        "<div id=\"cap\" style=\"display:none\"><img id=\"CImage\"></div>",
    );
    assert_eq!(dormant.expect("state"), MufgCaptchaState::Dormant);
    let required = MufgIntimeProvider::captcha_state("<div id=\"cap\"><img id=\"CImage\"></div>");
    assert_eq!(required.expect("state"), MufgCaptchaState::Required);
    // Reference without the recognized container: ambiguous, fail safe.
    let ambiguous = MufgIntimeProvider::captcha_state("<div>captcha text</div>");
    assert_eq!(ambiguous.expect("state"), MufgCaptchaState::Unknown);
}

#[test]
fn visible_captcha_ignores_unrelated_hidden_elements() {
    use sanket_allotment::MufgCaptchaState;
    let page = concat!(
        "<div id=\"unrelated\" style=\"display:none\">other</div>",
        "<div id=\"captcha\"><img id=\"CImage\"></div>"
    );
    assert_eq!(
        MufgIntimeProvider::captcha_state(page).expect("state"),
        MufgCaptchaState::Required
    );
}

#[test]
fn uppercase_captcha_text_without_structure_is_unknown() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state("<div>CAPTCHA validation</div>").expect("state"),
        MufgCaptchaState::Unknown
    );
}

#[test]
fn malformed_captcha_container_is_unknown() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state("<div id=\"captcha\"><img id=\"CImage\">")
            .expect("state"),
        MufgCaptchaState::Unknown
    );
}

#[test]
fn current_live_nested_captcha_wrapper_is_dormant() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state(&captcha_case("current_live_dormant")).expect("state"),
        MufgCaptchaState::Dormant
    );
}

#[test]
fn current_nested_wrapper_is_required_when_explicitly_visible() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state(&captcha_case("synthetic_required_same_wrapper"))
            .expect("state"),
        MufgCaptchaState::Required
    );
}

#[test]
fn current_wrapper_without_div_visibility_evidence_is_unknown() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state(&captcha_case("ambiguous_wrapper")).expect("state"),
        MufgCaptchaState::Unknown
    );
}

#[test]
fn unrelated_hidden_element_does_not_hide_visible_current_wrapper() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state(&captcha_case("unrelated_hidden_element"))
            .expect("state"),
        MufgCaptchaState::Required
    );
}

#[test]
fn current_wrapper_attributes_are_ascii_case_and_space_insensitive() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state(&captcha_case("mixed_case_dormant")).expect("state"),
        MufgCaptchaState::Dormant
    );
}

#[test]
fn unrecognized_current_captcha_structure_stays_unknown() {
    use sanket_allotment::MufgCaptchaState;
    assert_eq!(
        MufgIntimeProvider::captcha_state(&captcha_case("structural_drift")).expect("state"),
        MufgCaptchaState::Unknown
    );
}

// ---------------------------------------------------------------
// Request construction (synthetic only)
// ---------------------------------------------------------------

#[test]
fn request_construction_carries_token_session_and_redaction() {
    use sanket_allotment::MufgEphemeralSession;
    let session = MufgEphemeralSession::new(
        "session-1",
        "cookie-synthetic",
        "token-synthetic",
        "2026-8-28T18:09:00Z",
        Some("2026-08-28T18:14:00Z".into()),
    )
    .expect("session");
    let request =
        MufgIntimeProvider::build_lookup_request(&session, "11926", "2026-08-28T18:10:00Z")
            .expect("request");
    assert_eq!(
        request.endpoint,
        "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx/SearchOnPan"
    );
    assert_eq!(request.method, "POST");
    assert_eq!(request.content_type, "application/json; charset=utf-8");
    assert!(request.headers.iter().any(|h| h == "Cookie: [REDACTED]"));
    assert!(
        request
            .headers
            .iter()
            .any(|h| h == "Content-Type: application/json; charset=utf-8")
    );
    let body: serde_json::Value = serde_json::from_str(&request.body).expect("JSON request");
    assert_eq!(body["clientid"], "11926");
    assert_eq!(body["PAN"], "[SYNTHETIC_LOOKUP]");
    assert_eq!(body["IFSC"], "");
    assert_eq!(body["CHKVAL"], "1");
    assert_eq!(body["token"], "token-synthetic");
    assert_eq!(body.as_object().expect("object").len(), 5);
    assert!(!request.debug().contains("cookie-synthetic"));
    assert!(!request.debug().contains("token-synthetic"));
}

#[test]
fn current_json_token_contract_is_parsed_and_encrypted() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/session.json")).unwrap();
    let parsed_token = MufgIntimeProvider::parse_generated_token(
        &serde_json::to_string(&cases["token_current_json"]).unwrap(),
    )
    .expect("current token");
    assert_eq!(parsed_token, "123456789");
    assert_eq!(
        MufgIntimeProvider::encrypt_request_token(&parsed_token).expect("encrypted token"),
        "tmcrlyWVgO/ANL4vgSjBPA=="
    );
}

#[test]
#[ignore = "identifier-free live MUFG contract precheck"]
fn live_public_precheck_issue_11926_without_investor_identifier() {
    use sanket_allotment::MufgCaptchaState;

    let precheck = MufgIntimeProvider::new()
        .identifier_free_precheck("11926", "Symbiotec Pharmalab Limited")
        .expect("public precheck");
    assert_eq!(precheck.issue.provider_issue_id, "11926");
    assert_eq!(
        precheck.issue.display_name,
        "Symbiotec Pharmalab Limited - IPO"
    );
    assert!(matches!(
        precheck.captcha_state,
        MufgCaptchaState::Absent | MufgCaptchaState::Dormant
    ));
    assert!(precheck.tls_http_healthy);
    assert!(precheck.session_bootstrap_succeeded);
    assert!(precheck.request_token_obtained);
    assert_eq!(
        precheck.lookup_endpoint,
        "https://in.mpms.mufg.com/Initial_Offer/IPO.aspx/SearchOnPan"
    );
    println!("MUFG_CONTRACT=PASS");
    println!("ISSUE_11926=AVAILABLE");
    println!("CAPTCHA={:?}", precheck.captcha_state);
    println!("SESSION_BOOTSTRAP=PASS");
    println!("REQUEST_TOKEN=PASS");
    println!("REAL_PAN_LOOKUP=NOT_EXECUTED");
}

#[test]
fn parser_allots_only_on_structured_proof() {
    let result = parse_result(&parse_case("allotted")).expect("structured result");
    assert_eq!(result.status(), NormalizedAllotmentStatus::Allotted);
    assert_eq!(result.allotted_shares(), Some(35));
    assert_eq!(
        result.provenance(),
        ProviderResultProvenance::ConfirmedProviderResponse
    );
}

#[test]
fn parser_never_copies_member_fields() {
    let result = parse_result(&parse_case("allotted")).expect("structured result");
    assert_eq!(result.provider_reference(), None);
    assert_eq!(result.safe_message(), None);
}

#[test]
fn zero_allot_is_not_allotted_only_with_proof() {
    let result = parse_result(&parse_case("not_allotted")).expect("structured result");
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotAllotted);
    assert_eq!(result.allotted_shares(), None);
}

#[test]
fn empty_allot_is_pending() {
    let result = parse_result(&parse_case("pending")).expect("structured result");
    assert_eq!(result.status(), NormalizedAllotmentStatus::Pending);
}

#[test]
fn message_row_no_record_is_not_found() {
    let result = parse_result(&parse_case("not_found")).expect("structured result");
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotFound);
}

#[test]
fn specific_no_record_semantics_precede_generic_retry_text() {
    let result = parse_result(&parse_case("not_found_try_again")).expect("structured result");
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotFound);
}

#[test]
fn unknown_xml_message_fails_closed() {
    let err = parse_result(&parse_case("unknown")).expect_err("no record rows");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn malformed_allot_fails_closed() {
    let err = parse_result(&parse_case("malformed")).expect_err("garbled ALLOT");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn session_expired_is_operational_not_financial() {
    let err = parse_result(&parse_case("session_expired")).expect_err("session expired");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::RetryableError);
}

#[test]
fn token_invalid_is_operational_not_financial() {
    let err = parse_result(&parse_case("token_invalid")).expect_err("invalid token");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::RetryableError);
}

#[test]
fn captcha_active_maps_to_needs_human() {
    let err = parse_result(&parse_case("captcha_active")).expect_err("captcha");
    assert_eq!(
        err.to_status(),
        NormalizedAllotmentStatus::NeedsHumanVerification
    );
}

#[test]
fn captcha_dormant_page_does_not_require_verification() {
    let page = parse_case("captcha_dormant_page");
    let state =
        sanket_allotment::MufgIntimeProvider::captcha_state(&page).expect("captcha state parsed");
    assert_eq!(state, sanket_allotment::MufgCaptchaState::Dormant);
}

#[test]
fn captcha_active_page_requires_verification() {
    let page = parse_case("captcha_active_page");
    let state =
        sanket_allotment::MufgIntimeProvider::captcha_state(&page).expect("captcha state parsed");
    assert_eq!(state, sanket_allotment::MufgCaptchaState::Required);
}

#[test]
fn captcha_absent_page_is_absent() {
    let state =
        sanket_allotment::MufgIntimeProvider::captcha_state("<html><body>hello</body></html>")
            .expect("captcha state parsed");
    assert_eq!(state, sanket_allotment::MufgCaptchaState::Absent);
}

#[test]
fn rate_limited_is_operational() {
    let err = parse_result(&parse_case("rate_limited")).expect_err("rate limited");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::RateLimited);
}

#[test]
fn provider_unavailable_is_operational() {
    let err = parse_result(&parse_case("provider_unavailable")).expect_err("unavailable");
    assert_eq!(
        err.to_status(),
        NormalizedAllotmentStatus::ProviderUnavailable
    );
}

#[test]
fn drifted_structure_fails_closed() {
    let err = parse_result(&parse_case("drifted")).expect_err("drifted");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn wrong_fingerprint_is_rejected() {
    let body = parse_case("allotted");
    let err = sanket_allotment::MufgIntimeProvider::parse_result_body(
        &body,
        true,
        "2026-08-28T18:09:00Z",
        "mufg-result-drifted-v9",
    )
    .expect_err("fingerprint mismatch");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::Unknown);
}

#[test]
fn unattended_check_fails_closed_pending_transport() {
    let provider = mufg_provider();
    let pan = Pan::parse("ABCDE1234F").expect("synthetic PAN");
    let err = provider
        .check_allotment(&lookup_context(), &pan)
        .expect_err("no live transport in Gate 4D scope");
    // Session/token transport is a later slice: fail closed, never guess.
    let status = err.to_status();
    assert_ne!(status, NormalizedAllotmentStatus::Allotted);
    assert_ne!(status, NormalizedAllotmentStatus::NotAllotted);
}

#[test]
fn implemented_transport_requires_runtime_permit() {
    let provider = mufg_provider();
    let pan = Pan::parse("ABCDE1234F").expect("synthetic PAN");
    let err = provider
        .check_allotment(&lookup_context(), &pan)
        .expect_err("direct MUFG lookup must remain deny-by-default");
    assert_eq!(err.to_status(), NormalizedAllotmentStatus::RetryableError);
}

#[test]
fn health_reflects_capability_not_http_200() {
    let provider = mufg_provider();
    // Deterministic session/token transport is not yet proven unattended:
    // capability-honest Degraded, never Available-on-HTTP-200.
    assert_eq!(provider.health(), ProviderHealth::Degraded);
}

#[test]
fn supports_only_mufg_registrar_ids() {
    let provider = mufg_provider();
    assert!(provider.supports(&lookup_context().issue));
    let mut other = lookup_context();
    other.issue.registrar_id = "bigshare".into();
    assert!(!provider.supports(&other.issue));
}

// ---------------------------------------------------------------
// Token extraction + session bootstrap
// ---------------------------------------------------------------

#[test]
fn token_extraction_requires_exactly_one_nonempty_hidtoken() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/session.json")).unwrap();
    let extracted =
        MufgIntimeProvider::parse_token_response(cases["token_ok"]["d"].as_str().unwrap())
            .expect("token");
    assert_eq!(extracted, "[SYNTHETIC_TOKEN]");
    assert!(
        MufgIntimeProvider::parse_token_response(cases["token_missing"]["d"].as_str().unwrap())
            .is_err()
    );
    assert!(
        MufgIntimeProvider::parse_token_response(cases["token_empty"]["d"].as_str().unwrap())
            .is_err()
    );
    // Two tokens in one response is drift, not pick-first-wins.
    assert!(
        MufgIntimeProvider::parse_token_response(cases["token_duplicate"]["d"].as_str().unwrap())
            .is_err()
    );
}

#[test]
fn token_extraction_is_scoped_to_hidtoken_input() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/session.json")).unwrap();
    assert_eq!(
        MufgIntimeProvider::parse_token_response(
            cases["token_unrelated_before"]["d"].as_str().unwrap()
        )
        .expect("valid hidToken after unrelated input"),
        "[SYNTHETIC_TOKEN]"
    );
    assert!(
        MufgIntimeProvider::parse_token_response(cases["token_malformed"]["d"].as_str().unwrap())
            .is_err()
    );
}

#[test]
fn token_never_enters_debug_output() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/session.json")).unwrap();
    let extracted =
        MufgIntimeProvider::parse_token_response(cases["token_ok"]["d"].as_str().unwrap())
            .expect("token");
    let session = sanket_allotment::MufgEphemeralSession::new(
        "session-1",
        "cookie-synthetic",
        extracted,
        "2026-08-28T18:09:00Z",
        Some("2026-08-28T18:14:00Z".into()),
    )
    .expect("session");
    assert!(!format!("{session:?}").contains("SYNTHETIC_TOKEN"));
}

#[test]
fn bootstrap_captcha_states_from_session_fixtures() {
    use sanket_allotment::MufgCaptchaState;
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/session.json")).unwrap();
    let dormant =
        MufgIntimeProvider::captcha_state(cases["captcha_dormant_page"]["d"].as_str().unwrap())
            .expect("state");
    assert_eq!(dormant, MufgCaptchaState::Dormant);
    let required =
        MufgIntimeProvider::captcha_state(cases["captcha_required_page"]["d"].as_str().unwrap())
            .expect("state");
    assert_eq!(required, MufgCaptchaState::Required);
}

// ---------------------------------------------------------------
// Restart semantics — fresh session, never restored secrets
// ---------------------------------------------------------------

#[test]
fn restart_needs_fresh_session_expired_rejected_for_lookup() {
    use sanket_allotment::MufgEphemeralSession;
    let fresh = MufgEphemeralSession::new(
        "session-new",
        "[SYNTHETIC_COOKIE]",
        "[SYNTHETIC_TOKEN]",
        "2026-08-28T18:20:00Z",
        Some("2026-08-28T18:50:00Z".into()),
    )
    .expect("fresh session");
    assert!(!fresh.is_expired("2026-08-28T18:21:00Z"));

    // Expired session cannot build a lookup; refresh is required.
    let err = MufgIntimeProvider::build_lookup_request(
        &fresh,
        "11926",
        "2026-08-28T19:00:00Z", // past expiry
    );
    assert!(err.is_err());
}

#[test]
fn restart_transitions_are_typed_not_guessed() {
    use sanket_allotment::AllotmentCheckJob;
    use sanket_allotment::AllotmentJobStatus;
    let mut job = AllotmentCheckJob::create(
        "job-1",
        "app-1",
        "sess-1",
        "SYNTHETIC ALPHA LIMITED",
        "mufg intime",
        "MUFG Intime India",
        "mufg-intime-live",
    )
    .expect("job");

    // Restart with verification active: PreparingProviderSession →
    // VerificationRequiredRefresh (stale secrets are never restored).
    job.transition_to(AllotmentJobStatus::PreparingProviderSession)
        .expect("to preparing");
    job.transition_to(AllotmentJobStatus::VerificationRequiredRefresh)
        .expect("to refresh");
    assert_eq!(
        job.status(),
        AllotmentJobStatus::VerificationRequiredRefresh
    );

    // After the member re-verifies: refresh → fresh session preparation.
    job.transition_to(AllotmentJobStatus::PreparingProviderSession)
        .expect("back to preparing");
    assert_eq!(job.status(), AllotmentJobStatus::PreparingProviderSession);

    // Illegal shortcuts fail: refresh must not jump straight to Running.
    let mut bad = AllotmentCheckJob::create(
        "job-2",
        "app-1",
        "sess-1",
        "SYNTHETIC ALPHA LIMITED",
        "mufg intime",
        "MUFG Intime India",
        "mufg-intime-live",
    )
    .expect("job");
    bad.transition_to(AllotmentJobStatus::PreparingProviderSession)
        .expect("to preparing");
    bad.transition_to(AllotmentJobStatus::VerificationRequiredRefresh)
        .expect("to refresh");
    assert!(bad.transition_to(AllotmentJobStatus::Running).is_err());
}
