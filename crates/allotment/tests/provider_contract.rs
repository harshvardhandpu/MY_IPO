use sanket_allotment::{
    AllotmentCheckAttempt, AllotmentCheckJob, AllotmentJobStatus, AllotmentProvider, AttemptStatus,
    BackgroundExecution, FixtureKfintechProvider, HumanVerificationChallenge,
    HumanVerificationRequirement, HumanVerificationStatus, HumanVerificationType,
    IssueDiscoveryMode, LookupKeyKind, ManualReportedOutcome, ManualResultInput,
    NegativeResultProof, NormalizedAllotmentStatus, ProviderContinuationReference, ProviderId,
    ProviderRegistry, ProviderResultProvenance, ProviderTransportKind, SanitizedFixtureProvenance,
    SessionRequirement,
};

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
fn implemented_live_adapter_does_not_authorize_real_lookup() {
    assert!(!sanket_allotment::real_investor_lookup_allowed());
}
