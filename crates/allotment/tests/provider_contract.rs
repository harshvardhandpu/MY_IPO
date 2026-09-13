use sanket_allotment::{
    AllotmentCheckAttempt, AllotmentCheckJob, AllotmentJobStatus, AllotmentProvider,
    AllotmentResolutionState, AttemptStatus, BackgroundExecution, BigshareProvider,
    ConfirmedProviderIssue, FixtureKfintechProvider, HumanVerificationChallenge, HumanVerificationRequirement,
    HumanVerificationStatus, HumanVerificationType, IssueDiscoveryMode, KfintechProvider,
    LookupKeyKind, ManualReportedOutcome, ManualResultInput, MufgIntimeProvider,
    NegativeResultProof, NormalizedAllotmentStatus, PositiveResultProof,
    ProviderContinuationReference, ProviderId,
    ProviderRegistry, ProviderResultProvenance, ProviderTransportKind, SanitizedFixtureProvenance,
    SessionRequirement,
};

fn confirmed_issue(provider_id: &str) -> ConfirmedProviderIssue {
    ConfirmedProviderIssue::new(
        provider_id,
        "11927",
        "SYNTHETIC ALPHA LIMITED",
        "SYNTHETIC ALPHA LIMITED",
    )
    .unwrap()
}

#[test]
fn registry_alias_resolution_is_unified_and_fails_closed() {
    // Gate 4F condition E: one canonical policy for both resolvers.
    use sanket_allotment::ProviderId;

    // Canonical ids resolve in BOTH resolvers to the same provider.
    for (input, expected) in [
        ("kfintech", ProviderId::KfintechLive),
        ("bigshare", ProviderId::BigshareLive),
        ("mufg_intime", ProviderId::MufgIntimeLive),
    ] {
        assert_eq!(ProviderRegistry::resolve(input), Some(expected));
        let d = ProviderRegistry::resolve_registrar(input)
            .unwrap_or_else(|| panic!("resolve_registrar must know {input}"));
        assert_eq!(d.provider_id, expected, "{input}");
    }

    // Accepted aliases (space and underscore forms) normalize identically
    // in BOTH resolvers — no divergent alias sets.
    for alias in [
        "KFintech",
        "KFIN Technologies",
        "kfin technologies",
        "Bigshare Services",
        "bigshare services",
        "Link Intime",
        "link intime",
        "MUFG Intime",
        "mufg intime",
    ] {
        let via_resolve = ProviderRegistry::resolve(alias);
        let via_registrar = ProviderRegistry::resolve_registrar(alias);
        assert_eq!(
            via_resolve,
            via_registrar.map(|d| d.provider_id),
            "resolve and resolve_registrar must agree on {alias:?}"
        );
        assert!(via_resolve.is_some(), "alias {alias:?} must resolve");
    }

    // Unknown / unsupported / ambiguous: FAIL CLOSED in both.
    for unknown in [
        "unknown registrar",
        "acme kfin-like services",
        "linkintime",
        "symbiotic",
        "",
    ] {
        assert!(
            ProviderRegistry::resolve(unknown).is_none(),
            "resolve must fail closed on {unknown:?}"
        );
        assert!(
            ProviderRegistry::resolve_registrar(unknown).is_none(),
            "resolve_registrar must fail closed on {unknown:?}"
        );
    }

    // Explicit provider ids (persisted jobs) still resolve.
    assert_eq!(
        ProviderRegistry::resolve("kfintech-live"),
        Some(ProviderId::KfintechLive)
    );
    assert_eq!(
        ProviderRegistry::resolve("bigshare-live"),
        Some(ProviderId::BigshareLive)
    );
    assert_eq!(
        ProviderRegistry::resolve("mufg-intime-live"),
        Some(ProviderId::MufgIntimeLive)
    );
    assert_eq!(
        ProviderRegistry::resolve("kfintech-fixture"),
        Some(ProviderId::KfintechFixture)
    );
    // But provider ids are NOT registrar descriptors (no descriptor row).
    assert!(ProviderRegistry::resolve_registrar("kfintech-live").is_none());
}

#[test]
fn shared_capabilities_and_registry_are_typed() {
    let capabilities = FixtureKfintechProvider.capabilities();
    assert_eq!(capabilities.issue_discovery, IssueDiscoveryMode::None);
    assert!(capabilities.supports(LookupKeyKind::Pan));
    assert_eq!(capabilities.session, SessionRequirement::None);
    assert_eq!(capabilities.transport, ProviderTransportKind::Http);
    assert_eq!(capabilities.background, BackgroundExecution::Unattended);
    assert_eq!(
        capabilities.human_verification,
        HumanVerificationRequirement::None
    );

    assert_eq!(
        ProviderRegistry::resolve("kfintech"),
        Some(ProviderId::KfintechLive)
    );
    assert_eq!(
        ProviderRegistry::resolve("link intime"),
        Some(ProviderId::MufgIntimeLive)
    );
    assert_eq!(ProviderRegistry::resolve("unknown registrar"), None);
}

#[test]
fn not_allotted_requires_all_five_negative_proof_facts() {
    assert!(NegativeResultProof::new(true, true, true, true, false).is_err());
    let proof = NegativeResultProof::new(true, true, true, true, true).unwrap();
    let result = sanket_allotment::ProviderAllotmentResult::confirmed_not_allotted(
        proof,
        "2026-08-29T00:00:00Z",
        "sha256:fixture-contract",
    );
    assert_eq!(result.status(), NormalizedAllotmentStatus::NotAllotted);
    assert_eq!(
        result.provenance(),
        ProviderResultProvenance::ConfirmedProviderResponse
    );
}

#[test]
fn positive_proof_requires_confirmed_issue_identity() {
    let issue = ConfirmedProviderIssue::new(
        "kfintech-live",
        "11927",
        "Symbiotec Pharmalab Limited - IPO",
        "Symbiotec Pharmalab Limited - IPO",
    )
    .unwrap();
    assert!(PositiveResultProof::new(&issue, true, true, true).is_ok());
    assert!(ConfirmedProviderIssue::new(
        "mufg-intime-live",
        "11927",
        "Symbiotec Pharmalab Limited - IPO",
        "Symbiotec Pharmalab Limited",
    )
    .is_ok());
    assert!(
        ConfirmedProviderIssue::new(
            "kfintech-live",
            "11927",
            "Symbiotec Pharmalab Limited - IPO",
            "Wrong issue - IPO",
        )
        .is_err()
    );
}

#[test]
fn manual_negative_remains_manual_result() {
    let mut attempt = AllotmentCheckAttempt::new("attempt", "job", "account").unwrap();
    attempt.apply_manual(&ManualResultInput {
        allotted_lots: None,
        allotted_shares: None,
        explicit_not_allotted: true,
        note: None,
        actor_member_id: "owner".into(),
    });
    assert_eq!(attempt.status(), AttemptStatus::ManualResult);
    assert_eq!(
        attempt.manual_reported_outcome(),
        Some(ManualReportedOutcome::NotAllotted)
    );
}

#[test]
fn retry_policy_is_provider_specific() {
    let fixture = sanket_allotment::ProviderRatePolicy::for_provider(ProviderId::KfintechFixture);
    let kfintech = sanket_allotment::ProviderRatePolicy::for_provider(ProviderId::KfintechLive);
    let bigshare = sanket_allotment::ProviderRatePolicy::for_provider(ProviderId::BigshareLive);

    assert_eq!(fixture.min_interval_ms, 0);
    assert_eq!(fixture.max_attempts, 1);
    assert_eq!(kfintech.max_attempts, 3);
    assert_eq!(bigshare.max_attempts, 1);
}

#[test]
fn limiter_wiring_selects_policy_per_provider() {
    // Gate 4F condition B: one limiter, per-provider policies — no global
    // Default spacing for every registrar.
    use std::time::Instant;

    let limiter = sanket_allotment::ProviderRateLimiter::new(Default::default());
    for kind in [
        ProviderId::KfintechFixture,
        ProviderId::KfintechLive,
        ProviderId::BigshareLive,
        ProviderId::MufgIntimeLive,
    ] {
        limiter.set_policy(
            kind.as_str(),
            sanket_allotment::ProviderRatePolicy::for_provider(kind),
        );
    }

    // Registered policies are distinct and match the per-provider values.
    assert_eq!(limiter.policy_for("kfintech-fixture").min_interval_ms, 0);
    assert_eq!(limiter.policy_for("kfintech-fixture").max_attempts, 1);
    assert_eq!(limiter.policy_for("kfintech-live").min_interval_ms, 1_500);
    assert_eq!(limiter.policy_for("kfintech-live").max_attempts, 3);
    assert_eq!(limiter.policy_for("bigshare-live").max_attempts, 1);
    assert_eq!(limiter.policy_for("mufg-intime-live").max_attempts, 2);
    // Distinctness across the three live registrars is the point.
    let k = limiter.policy_for("kfintech-live");
    let b = limiter.policy_for("bigshare-live");
    let m = limiter.policy_for("mufg-intime-live");
    assert!(
        k != b && b != m && k != m,
        "live policies must not collapse"
    );

    // wait_turn uses the provider's own spacing: fixture (0ms) fires twice
    // immediately; a 1500ms provider's second turn must report wait.
    let t0 = Instant::now();
    limiter.wait_turn("kfintech-fixture");
    limiter.wait_turn("kfintech-fixture");
    assert!(
        t0.elapsed().as_millis() < 500,
        "fixture must not be rate-limited"
    );

    let limiter2 = sanket_allotment::ProviderRateLimiter::new(Default::default());
    for kind in [
        ProviderId::KfintechLive,
        ProviderId::BigshareLive,
        ProviderId::MufgIntimeLive,
    ] {
        limiter2.set_policy(
            kind.as_str(),
            sanket_allotment::ProviderRatePolicy::for_provider(kind),
        );
    }
    limiter2.wait_turn("kfintech-live");
    let t1 = Instant::now();
    limiter2.wait_turn("kfintech-live");
    assert!(
        t1.elapsed().as_millis() >= 1_400,
        "second kfintech-live call must be spaced ~1500ms, took {:?}ms",
        t1.elapsed()
    );
}

#[test]
fn challenge_is_safe_metadata_and_job_can_refresh_after_restart() {
    let challenge = HumanVerificationChallenge::new(
        "challenge-1",
        "bigshare-live",
        "job-1",
        "attempt-1",
        "account-1",
        HumanVerificationType::Captcha,
        HumanVerificationStatus::Required,
        "server-1",
        "2026-08-29T00:00:00Z",
        Some("2026-08-29T00:05:00Z".into()),
        ProviderContinuationReference::new("continuation-1").unwrap(),
    )
    .unwrap();
    let json = serde_json::to_string(&challenge).unwrap();
    for forbidden in [
        "cookie",
        "captcha_answer",
        "request_token",
        "response_body",
        "pan",
    ] {
        assert!(!json.to_ascii_lowercase().contains(forbidden));
    }
    assert!(!json.contains("continuation-1"));
    assert!(format!("{:?}", challenge).contains("[REDACTED]"));
    assert!(ProviderContinuationReference::new("session-token-abc").is_err());
    assert!(ProviderContinuationReference::new("eyJhbGciOiJIUzI1NiJ9").is_err());
    assert!(ProviderContinuationReference::new("continuation_2").is_ok());

    let mut job = AllotmentCheckJob::create(
        "job-1",
        "app-1",
        "session-1",
        "IPO",
        "bigshare",
        "Bigshare",
        "bigshare-live",
    )
    .unwrap();
    job.transition_to(AllotmentJobStatus::PreparingProviderSession)
        .unwrap();
    job.transition_to(AllotmentJobStatus::VerificationRequiredRefresh)
        .unwrap();
}

#[test]
fn sanitized_fixture_provenance_is_complete() {
    assert!(
        SanitizedFixtureProvenance::new(
            "kfintech-live",
            "https://ipostatus.kfintech.com/status",
            "2026-08-29T00:00:00Z",
            "negative-result",
            "bad-hash",
            Some("kfintech-result:v1".into()),
        )
        .is_err()
    );
    let fixture = SanitizedFixtureProvenance::new(
        "kfintech-live",
        "https://ipostatus.kfintech.com/status",
        "2026-08-29T00:00:00Z",
        "negative-result",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        Some("kfintech-result:v1".into()),
    )
    .unwrap();
    let json = serde_json::to_value(fixture).unwrap();
    assert_eq!(json["sanitized"], true);
    assert_eq!(json["fixture_type"], "negative-result");
    assert_eq!(json["structural_fingerprint"], "kfintech-result:v1");
}

#[test]
fn runtime_lookup_permit_requires_safe_scope() {
    use sanket_allotment::RealInvestorLookupPermit;

    assert!(RealInvestorLookupPermit::new("", "app-1", "mufg-intime-live").is_err());
    assert!(
        RealInvestorLookupPermit::new("auth-1", "app-1", "mufg-intime-live")
            .unwrap()
            .matches("app-1", "mufg-intime-live")
    );
}

#[test]
fn registrar_registry_routes_only_explicit_supported_ids() {
    let kfin = ProviderRegistry::resolve_registrar("kfintech").unwrap();
    let bigshare = ProviderRegistry::resolve_registrar("bigshare").unwrap();
    let mufg = ProviderRegistry::resolve_registrar("mufg_intime").unwrap();

    assert_eq!(kfin.provider_id, ProviderId::KfintechLive);
    assert_eq!(bigshare.provider_id, ProviderId::BigshareLive);
    assert_eq!(mufg.provider_id, ProviderId::MufgIntimeLive);
    assert!(ProviderRegistry::resolve_registrar("unknown registrar").is_none());
    assert!(ProviderRegistry::resolve_registrar("Acme KFin-like Services").is_none());
}

#[test]
fn mixed_account_states_are_partially_complete_without_hiding_finals() {
    assert_eq!(
        AllotmentJobStatus::from_attempt_statuses(&[
            AttemptStatus::Allotted,
            AttemptStatus::NeedsHumanVerification,
        ]),
        AllotmentJobStatus::PartiallyComplete
    );
    assert_eq!(
        AllotmentJobStatus::from_attempt_statuses(&[
            AttemptStatus::Allotted,
            AttemptStatus::NotAllotted,
            AttemptStatus::NotFound,
            AttemptStatus::ManualResult,
        ]),
        AllotmentJobStatus::PartiallyComplete
    );
    assert_eq!(
        AllotmentJobStatus::from_attempt_statuses(&[AttemptStatus::NeedsHumanVerification]),
        AllotmentJobStatus::PartiallyComplete
    );
}

#[test]
fn resolution_state_keeps_operational_failures_and_unknowns_unresolved() {
    assert_eq!(
        AllotmentResolutionState::from_attempt_status("PROVIDER_UNAVAILABLE", true, false),
        AllotmentResolutionState::RetryableProviderFailure
    );
    assert_eq!(
        AllotmentResolutionState::from_attempt_status("PROVIDER_UNAVAILABLE", false, false),
        AllotmentResolutionState::Unresolved
    );
    assert_eq!(
        AllotmentResolutionState::from_attempt_status("NOT_FOUND", false, false),
        AllotmentResolutionState::Unresolved
    );
    for status in ["UNKNOWN", "ISSUE_NOT_AVAILABLE", "RESPONSE_CHANGED"] {
        assert_eq!(
            AllotmentResolutionState::from_attempt_status(status, false, false),
            AllotmentResolutionState::Unresolved,
            "{status} must remain unresolved"
        );
    }
    assert_eq!(
        AllotmentResolutionState::from_attempt_status("NEEDS_HUMAN_VERIFICATION", false, false),
        AllotmentResolutionState::InteractionRequired
    );
    assert_eq!(
        AllotmentResolutionState::from_attempt_status("MANUAL_RESULT", false, true),
        AllotmentResolutionState::ManualConfirmed
    );
}

#[test]
fn issue_unavailable_and_response_change_are_distinct_unresolved_outcomes() {
    assert_eq!(
        NormalizedAllotmentStatus::from_provider_text("requested issue not available"),
        NormalizedAllotmentStatus::IssueNotAvailable
    );
    assert_eq!(
        NormalizedAllotmentStatus::from_provider_text("provider response changed"),
        NormalizedAllotmentStatus::ResponseChanged
    );
    assert!(!NormalizedAllotmentStatus::IssueNotAvailable.is_final());
    assert!(!NormalizedAllotmentStatus::ResponseChanged.is_final());
}

#[test]
fn every_provider_rejects_a_stale_result_contract_as_response_changed() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/kfintech/cases.json")).unwrap();
    let kfintech = KfintechProvider::parse_result_body(
        &cases["allotted"].to_string(),
        Some(&confirmed_issue("kfintech-live")),
        "SYNTHETIC ALPHA LIMITED",
        "2026-08-29T12:00:00Z",
        "stale-contract",
    )
    .expect_err("stale KFintech contract");
    assert_eq!(
        kfintech.to_status(),
        NormalizedAllotmentStatus::ResponseChanged
    );

    let bigshare = BigshareProvider::parse_result_body(
        r#"{"d":{"Status":"NOTFOUND"}}"#,
        Some(&confirmed_issue("bigshare-live")),
        "SYNTHETIC ALPHA LIMITED",
        "2026-08-29T12:00:00Z",
        "stale-contract",
    )
    .expect_err("stale Bigshare contract");
    assert_eq!(
        bigshare.to_status(),
        NormalizedAllotmentStatus::ResponseChanged
    );

    let mufg = MufgIntimeProvider::parse_result_body(
        "{}",
        Some(&confirmed_issue("mufg-intime-live")),
        "SYNTHETIC ALPHA LIMITED",
        "2026-08-29T12:00:00Z",
        "stale-contract",
    )
    .expect_err("stale MUFG contract");
    assert_eq!(mufg.to_status(), NormalizedAllotmentStatus::ResponseChanged);
}

#[test]
fn confirmed_results_have_cross_provider_application_semantics() {
    let kfin_cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/kfintech/cases.json")).unwrap();
    let bigshare_cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bigshare/cases.json")).unwrap();
    let mufg_cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/cases.json")).unwrap();

    let results = [
        KfintechProvider::parse_result_body(
            &kfin_cases["allotted"].to_string(),
            Some(&confirmed_issue("kfintech-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "kfin-result-data-array-v1",
        )
        .unwrap(),
        BigshareProvider::parse_result_body(
            &bigshare_cases["ok_allotted"].to_string(),
            Some(&confirmed_issue("bigshare-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "bigshare-result-d-status-v1",
        )
        .unwrap(),
        MufgIntimeProvider::parse_result_body(
            &mufg_cases["allotted"].to_string(),
            Some(&confirmed_issue("mufg-intime-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "mufg-result-d-xml-table-v1",
        )
        .unwrap(),
    ];

    for result in results {
        assert_eq!(result.status(), NormalizedAllotmentStatus::Allotted);
        assert_eq!(result.allotted_shares(), Some(35));
        assert_eq!(
            result.provenance(),
            ProviderResultProvenance::ConfirmedProviderResponse
        );
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains("ABCDE1234F"));
        assert!(!serialized.to_ascii_lowercase().contains("cookie"));
        assert!(!serialized.to_ascii_lowercase().contains("request_token"));
    }
}

#[test]
fn only_confirmed_provider_negatives_normalize_to_not_allotted() {
    let kfin_cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/kfintech/cases.json")).unwrap();
    let bigshare_cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bigshare/cases.json")).unwrap();
    let mufg_cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/mufg/cases.json")).unwrap();

    let negatives = [
        KfintechProvider::parse_result_body(
            &kfin_cases["not_allotted"].to_string(),
            Some(&confirmed_issue("kfintech-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "kfin-result-data-array-v1",
        )
        .unwrap(),
        BigshareProvider::parse_result_body(
            &bigshare_cases["ok_not_allotted"].to_string(),
            Some(&confirmed_issue("bigshare-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "bigshare-result-d-status-v1",
        )
        .unwrap(),
        MufgIntimeProvider::parse_result_body(
            &mufg_cases["not_allotted"].to_string(),
            Some(&confirmed_issue("mufg-intime-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "mufg-result-d-xml-table-v1",
        )
        .unwrap(),
    ];
    assert!(
        negatives
            .iter()
            .all(|result| result.status() == NormalizedAllotmentStatus::NotAllotted)
    );

    assert_eq!(
        BigshareProvider::parse_result_body(
            &bigshare_cases["notfound"].to_string(),
            Some(&confirmed_issue("bigshare-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "bigshare-result-d-status-v1",
        )
        .unwrap()
        .status(),
        NormalizedAllotmentStatus::NotFound
    );
    assert_eq!(
        MufgIntimeProvider::parse_result_body(
            &mufg_cases["not_found"].to_string(),
            Some(&confirmed_issue("mufg-intime-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "mufg-result-d-xml-table-v1",
        )
        .unwrap()
        .status(),
        NormalizedAllotmentStatus::NotFound
    );
    assert!(matches!(
        KfintechProvider::parse_result_body(
            &kfin_cases["unknown"].to_string(),
            Some(&confirmed_issue("kfintech-live")),
        "SYNTHETIC ALPHA LIMITED",
            "2026-08-29T12:00:00Z",
            "kfin-result-data-array-v1",
        ),
        Err(sanket_allotment::ProviderError::ResponseChanged(_))
    ));
}
