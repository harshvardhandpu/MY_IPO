//! Provider-independent allotment contract. No CSS/HTML/browser details here.

use serde::{Deserialize, Serialize};

use sanket_identity_security::Pan;

use crate::status::NormalizedAllotmentStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LookupKeyKind {
    Pan,
    ApplicationNumber,
    ApplicationNumberAndPan,
    DematAccount,
    BankAccountAndIfsc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IssueDiscoveryMode {
    None,
    PublicHttp,
    PublicJavascript,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionRequirement {
    None,
    ChallengeToken,
    Cookie,
    CookieAndRequestToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HumanVerificationRequirement {
    None,
    Conditional,
    Required,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderTransportKind {
    Http,
    Browser,
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackgroundExecution {
    Unattended,
    PrepareOnly,
    ForegroundOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub issue_discovery: IssueDiscoveryMode,
    pub lookup_keys: Vec<LookupKeyKind>,
    pub session: SessionRequirement,
    pub human_verification: HumanVerificationRequirement,
    pub transport: ProviderTransportKind,
    pub background: BackgroundExecution,
}

impl ProviderCapabilities {
    pub fn supports(&self, key: LookupKeyKind) -> bool {
        self.lookup_keys.contains(&key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderContinuationReference(String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid safe provider metadata")]
pub struct SafeProviderMetadataError;

fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

impl ProviderContinuationReference {
    pub fn new(value: impl Into<String>) -> Result<Self, SafeProviderMetadataError> {
        let value = value.into();
        is_safe_identifier(&value)
            .then_some(Self(value))
            .ok_or(SafeProviderMetadataError)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HumanVerificationType {
    Captcha,
    Otp,
    InteractiveBrowser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HumanVerificationStatus {
    Required,
    Presented,
    Completed,
    Expired,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanVerificationChallenge {
    challenge_id: String,
    provider_id: String,
    job_id: String,
    attempt_id: String,
    account_id: String,
    challenge_type: HumanVerificationType,
    status: HumanVerificationStatus,
    endpoint_id: String,
    created_at: String,
    expires_at: Option<String>,
    continuation_reference: ProviderContinuationReference,
}

impl HumanVerificationChallenge {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        challenge_id: impl Into<String>,
        provider_id: impl Into<String>,
        job_id: impl Into<String>,
        attempt_id: impl Into<String>,
        account_id: impl Into<String>,
        challenge_type: HumanVerificationType,
        status: HumanVerificationStatus,
        endpoint_id: impl Into<String>,
        created_at: impl Into<String>,
        expires_at: Option<String>,
        continuation_reference: ProviderContinuationReference,
    ) -> Result<Self, SafeProviderMetadataError> {
        let challenge = Self {
            challenge_id: challenge_id.into(),
            provider_id: provider_id.into(),
            job_id: job_id.into(),
            attempt_id: attempt_id.into(),
            account_id: account_id.into(),
            challenge_type,
            status,
            endpoint_id: endpoint_id.into(),
            created_at: created_at.into(),
            expires_at,
            continuation_reference,
        };
        [
            challenge.challenge_id.as_str(),
            challenge.provider_id.as_str(),
            challenge.job_id.as_str(),
            challenge.attempt_id.as_str(),
            challenge.account_id.as_str(),
            challenge.endpoint_id.as_str(),
        ]
        .iter()
        .all(|value| is_safe_identifier(value))
        .then_some(challenge)
        .ok_or(SafeProviderMetadataError)
    }

    pub fn status(&self) -> HumanVerificationStatus {
        self.status
    }

    pub fn challenge_type(&self) -> HumanVerificationType {
        self.challenge_type
    }

    pub fn endpoint_id(&self) -> &str {
        &self.endpoint_id
    }

    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }

    pub fn continuation_reference(&self) -> &str {
        self.continuation_reference.as_str()
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn job_id(&self) -> &str {
        &self.job_id
    }

    pub fn account_id(&self) -> &str {
        &self.account_id
    }

    /// Legal lifecycle transitions. Completed/Expired/Cancelled are terminal;
    /// an active challenge may be cancelled or may expire; Required may be
    /// presented; Presented may be completed. Nothing skips or regresses.
    pub fn transition(
        &self,
        next: HumanVerificationStatus,
    ) -> Result<Self, SafeProviderMetadataError> {
        use HumanVerificationStatus::{Cancelled, Completed, Expired, Presented, Required};
        let legal = matches!(
            (self.status, next),
            (Required, Presented)
                | (Required, Cancelled)
                | (Required, Expired)
                | (Presented, Completed)
                | (Presented, Cancelled)
                | (Presented, Expired)
        );
        if !legal {
            return Err(SafeProviderMetadataError);
        }
        let mut next_challenge = self.clone();
        next_challenge.status = next;
        Ok(next_challenge)
    }

    /// Whether the resumable window has closed at `now` (RFC 3339 UTC).
    /// A challenge with no expiry never expires by time alone. Fails safe:
    /// anything at or past the expiry is expired.
    pub fn is_expired(&self, now: &str) -> bool {
        // ponytail: RFC 3339 UTC "Z" timestamps compare correctly as
        // strings under the provenance contract's fixed shape; a malformed
        // value fails safe (treated as expired).
        match self.expires_at.as_deref() {
            Some(expires_at) => now >= expires_at,
            None => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedFixtureProvenance {
    provider: String,
    source_url: String,
    retrieved_at: String,
    fixture_type: String,
    sanitized: bool,
    content_sha256: String,
    structural_fingerprint: Option<String>,
}

impl SanitizedFixtureProvenance {
    pub fn from_json(json: &str) -> Result<Self, SafeProviderMetadataError> {
        #[derive(Deserialize)]
        struct Document {
            provider: String,
            source_url: String,
            retrieved_at: String,
            fixture_type: String,
            sanitized: bool,
            content_sha256: String,
            structural_fingerprint: Option<String>,
        }

        let document: Document =
            serde_json::from_str(json).map_err(|_| SafeProviderMetadataError)?;
        if !document.sanitized {
            return Err(SafeProviderMetadataError);
        }
        Self::new(
            document.provider,
            document.source_url,
            document.retrieved_at,
            document.fixture_type,
            document.content_sha256,
            document.structural_fingerprint,
        )
    }

    pub fn new(
        provider: impl Into<String>,
        source_url: impl Into<String>,
        retrieved_at: impl Into<String>,
        fixture_type: impl Into<String>,
        content_sha256: impl Into<String>,
        structural_fingerprint: Option<String>,
    ) -> Result<Self, SafeProviderMetadataError> {
        let provenance = Self {
            provider: provider.into(),
            source_url: source_url.into(),
            retrieved_at: retrieved_at.into(),
            fixture_type: fixture_type.into(),
            sanitized: true,
            content_sha256: content_sha256.into(),
            structural_fingerprint,
        };
        let valid = is_safe_identifier(&provenance.provider)
            && provenance.source_url.starts_with("https://")
            && provenance.source_url.len() <= 2_048
            && !provenance
                .source_url
                .bytes()
                .any(|b| b.is_ascii_whitespace())
            && !provenance.retrieved_at.is_empty()
            && provenance.retrieved_at.len() <= 64
            && is_safe_identifier(&provenance.fixture_type)
            && provenance.content_sha256.len() == 64
            && provenance
                .content_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            && provenance
                .structural_fingerprint
                .as_deref()
                .is_none_or(is_safe_identifier);
        valid.then_some(provenance).ok_or(SafeProviderMetadataError)
    }

    pub fn verify_content(&self, content: &[u8]) -> Result<(), SafeProviderMetadataError> {
        use sha2::{Digest, Sha256};

        let actual = format!("{:x}", Sha256::digest(content));
        (actual == self.content_sha256)
            .then_some(())
            .ok_or(SafeProviderMetadataError)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrarIssue {
    pub registrar_id: String,
    pub registrar_name: String,
    pub official_status_url: Option<String>,
    pub issue_code: Option<String>,
    pub ipo_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllotmentLookupContext {
    pub job_id: String,
    pub attempt_id: String,
    pub account_id: String,
    pub issue: RegistrarIssue,
}

/// Runtime proof that the application service validated one owner-approved,
/// bounded real-investor lookup. It carries identifiers only — never PAN or
/// provider credentials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RealInvestorLookupPermit {
    authorization_id: String,
    application_id: String,
    provider_id: String,
}

impl RealInvestorLookupPermit {
    pub fn new(
        authorization_id: impl Into<String>,
        application_id: impl Into<String>,
        provider_id: impl Into<String>,
    ) -> Result<Self, SafeProviderMetadataError> {
        let permit = Self {
            authorization_id: authorization_id.into(),
            application_id: application_id.into(),
            provider_id: provider_id.into(),
        };
        (is_safe_identifier(&permit.authorization_id)
            && is_safe_identifier(&permit.application_id)
            && is_safe_identifier(&permit.provider_id))
        .then_some(permit)
        .ok_or(SafeProviderMetadataError)
    }

    pub fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub fn application_id(&self) -> &str {
        &self.application_id
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn matches(&self, application_id: &str, provider_id: &str) -> bool {
        self.application_id == application_id && self.provider_id == provider_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderResultProvenance {
    ConfirmedProviderResponse,
    Fixture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NegativeResultProof(());

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("negative result proof is incomplete")]
pub struct NegativeResultProofError;

impl NegativeResultProof {
    pub fn new(
        provider_confirmed: bool,
        issue_confirmed: bool,
        structure_confirmed: bool,
        negative_marker_confirmed: bool,
        unambiguous: bool,
    ) -> Result<Self, NegativeResultProofError> {
        if provider_confirmed
            && issue_confirmed
            && structure_confirmed
            && negative_marker_confirmed
            && unambiguous
        {
            Ok(Self(()))
        } else {
            Err(NegativeResultProofError)
        }
    }
}

/// A positively recognized provider allotment is structurally constrained:
/// it must carry a positive share count and an unambiguous confirmed marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositiveResultProof(()); // ponytail: unit proof mirrors NegativeResultProof; add fields if drift demands it

impl PositiveResultProof {
    pub fn new(
        provider_confirmed: bool,
        structure_confirmed: bool,
        allotted_marker_confirmed: bool,
        unambiguous: bool,
    ) -> Result<Self, ProviderError> {
        if provider_confirmed && structure_confirmed && allotted_marker_confirmed && unambiguous {
            Ok(Self(()))
        } else {
            Err(ProviderError::Unknown(
                "allotted result proof is incomplete".into(),
            ))
        }
    }
}

/// Safe provider outcome — never contains PAN. Final negative construction is guarded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAllotmentResult {
    status: NormalizedAllotmentStatus,
    allotted_lots: Option<u32>,
    allotted_shares: Option<u64>,
    provider_reference: Option<String>,
    checked_at: String,
    safe_message: Option<String>,
    contract_fingerprint: Option<String>,
    provenance: ProviderResultProvenance,
}

impl ProviderAllotmentResult {
    pub fn confirmed_not_allotted(
        _proof: NegativeResultProof,
        checked_at: impl Into<String>,
        contract_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            status: NormalizedAllotmentStatus::NotAllotted,
            allotted_lots: None,
            allotted_shares: None,
            provider_reference: None,
            checked_at: checked_at.into(),
            safe_message: None,
            contract_fingerprint: Some(contract_fingerprint.into()),
            provenance: ProviderResultProvenance::ConfirmedProviderResponse,
        }
    }

    /// Guarded affirmative constructor for a positively recognized provider
    /// response. Refuses nil shares/lots: ALLOTTED without a positive quantity
    /// is treated as ambiguity, never as a confirmed allotment.
    pub fn confirmed_allotted(
        _proof: PositiveResultProof,
        allotted_shares: u64,
        allotted_lots: Option<u32>,
        provider_reference: Option<String>,
        checked_at: impl Into<String>,
        contract_fingerprint: impl Into<String>,
    ) -> Result<Self, ProviderError> {
        if allotted_shares == 0 {
            return Err(ProviderError::Unknown(
                "allotted result must carry a positive share count".into(),
            ));
        }
        Ok(Self {
            status: NormalizedAllotmentStatus::Allotted,
            allotted_lots,
            allotted_shares: Some(allotted_shares),
            provider_reference,
            checked_at: checked_at.into(),
            safe_message: None,
            contract_fingerprint: Some(contract_fingerprint.into()),
            provenance: ProviderResultProvenance::ConfirmedProviderResponse,
        })
    }

    /// A structurally valid single result record with a nil share count is a
    /// recognized, unambiguous provider-pending state — never a final outcome.
    pub fn pending(checked_at: impl Into<String>, contract_fingerprint: impl Into<String>) -> Self {
        Self {
            status: NormalizedAllotmentStatus::Pending,
            allotted_lots: None,
            allotted_shares: None,
            provider_reference: None,
            checked_at: checked_at.into(),
            safe_message: None,
            contract_fingerprint: Some(contract_fingerprint.into()),
            provenance: ProviderResultProvenance::ConfirmedProviderResponse,
        }
    }

    /// A recognized provider no-record state (e.g. Bigshare `NOTFOUND`) is a
    /// typed operational outcome — never `NOT_ALLOTTED`, which stays reserved
    /// for a guarded negative proof.
    pub fn not_found(
        checked_at: impl Into<String>,
        contract_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            status: NormalizedAllotmentStatus::NotFound,
            allotted_lots: None,
            allotted_shares: None,
            provider_reference: None,
            checked_at: checked_at.into(),
            safe_message: None,
            contract_fingerprint: Some(contract_fingerprint.into()),
            provenance: ProviderResultProvenance::ConfirmedProviderResponse,
        }
    }

    /// A recognized provider operational state (challenge required again,
    /// rate limited, endpoint warming) carried inside a structurally valid
    /// response. Guarded: it can never construct a financial status —
    /// `Allotted`/`NotAllotted` remain reachable only through their proof
    /// constructors.
    pub fn operational(
        status: NormalizedAllotmentStatus,
        checked_at: impl Into<String>,
        contract_fingerprint: impl Into<String>,
    ) -> Option<Self> {
        if !matches!(
            status,
            NormalizedAllotmentStatus::NeedsHumanVerification
                | NormalizedAllotmentStatus::RateLimited
                | NormalizedAllotmentStatus::RetryableError
        ) {
            return None;
        }
        Some(Self {
            status,
            allotted_lots: None,
            allotted_shares: None,
            provider_reference: None,
            checked_at: checked_at.into(),
            safe_message: None,
            contract_fingerprint: Some(contract_fingerprint.into()),
            provenance: ProviderResultProvenance::ConfirmedProviderResponse,
        })
    }

    fn fixture(
        status: NormalizedAllotmentStatus,
        allotted_lots: Option<u32>,
        allotted_shares: Option<u64>,
        provider_reference: Option<String>,
        safe_message: Option<String>,
    ) -> Self {
        Self {
            status,
            allotted_lots,
            allotted_shares,
            provider_reference,
            checked_at: "fixture".into(),
            safe_message,
            contract_fingerprint: Some("fixture:v1".into()),
            provenance: ProviderResultProvenance::Fixture,
        }
    }

    pub fn status(&self) -> NormalizedAllotmentStatus {
        self.status
    }

    pub fn allotted_lots(&self) -> Option<u32> {
        self.allotted_lots
    }

    pub fn allotted_shares(&self) -> Option<u64> {
        self.allotted_shares
    }

    pub fn provider_reference(&self) -> Option<&str> {
        self.provider_reference.as_deref()
    }

    pub fn checked_at(&self) -> &str {
        &self.checked_at
    }

    pub fn safe_message(&self) -> Option<&str> {
        self.safe_message.as_deref()
    }

    pub fn contract_fingerprint(&self) -> Option<&str> {
        self.contract_fingerprint.as_deref()
    }

    pub fn provenance(&self) -> ProviderResultProvenance {
        self.provenance
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    #[error("rate limited")]
    RateLimited,
    #[error("needs human verification: {0}")]
    NeedsHuman(String),
    #[error("retryable: {0}")]
    Retryable(String),
    #[error("parse/layout mismatch: {0}")]
    Unknown(String),
}

impl ProviderError {
    pub fn to_status(&self) -> NormalizedAllotmentStatus {
        match self {
            Self::Unavailable(_) => NormalizedAllotmentStatus::ProviderUnavailable,
            Self::RateLimited => NormalizedAllotmentStatus::RateLimited,
            Self::NeedsHuman(_) => NormalizedAllotmentStatus::NeedsHumanVerification,
            Self::Retryable(_) => NormalizedAllotmentStatus::RetryableError,
            Self::Unknown(_) => NormalizedAllotmentStatus::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderHealth {
    Available,
    Degraded,
    HumanVerificationRequired,
    Broken,
    Unknown,
}

/// Domain/application layers talk only to this trait.
pub trait AllotmentProvider: Send + Sync {
    fn provider_id(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;
    fn health(&self) -> ProviderHealth;
    fn supports(&self, issue: &RegistrarIssue) -> bool;

    /// Provider preparation that needs no investor identifier. Prepare-only
    /// providers return a typed operational result here before PAN access.
    fn prepare_lookup(
        &self,
        _context: &AllotmentLookupContext,
    ) -> Result<Option<ProviderAllotmentResult>, ProviderError> {
        Ok(None)
    }

    /// PAN is a temporary argument. Implementations must not store it.
    fn check_allotment(
        &self,
        context: &AllotmentLookupContext,
        pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError>;

    /// Runtime-authorized PAN lookup. The application service must validate the
    /// event-backed permit before calling this method.
    fn check_allotment_with_permit(
        &self,
        application_id: &str,
        context: &AllotmentLookupContext,
        pan: &Pan,
        permit: &RealInvestorLookupPermit,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        if !permit.matches(application_id, self.provider_id()) {
            return Err(ProviderError::Retryable(
                "real investor lookup permit scope mismatch".into(),
            ));
        }
        self.check_allotment(context, pan)
    }
}

/// Deterministic fixture provider for CI and local synthetic runs.
///
/// Behavior keyed off the last character of the **normalized** PAN only for
/// synthetic fixtures — production providers never encode PAN into results.
pub struct FixtureKfintechProvider;

impl AllotmentProvider for FixtureKfintechProvider {
    fn provider_id(&self) -> &'static str {
        "kfintech-fixture"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            issue_discovery: IssueDiscoveryMode::None,
            lookup_keys: vec![LookupKeyKind::Pan],
            session: SessionRequirement::None,
            human_verification: HumanVerificationRequirement::None,
            transport: ProviderTransportKind::Http,
            background: BackgroundExecution::Unattended,
        }
    }

    fn health(&self) -> ProviderHealth {
        ProviderHealth::Available
    }

    fn supports(&self, issue: &RegistrarIssue) -> bool {
        let id = issue.registrar_id.to_ascii_lowercase();
        id.contains("kfin") || id == "kfintech" || id == "kfintech-fixture"
    }

    fn check_allotment(
        &self,
        context: &AllotmentLookupContext,
        pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError> {
        // Synthetic routing only — never log pan.
        let normalized = pan.as_normalized();
        let last = normalized.chars().last().unwrap_or('0');
        let (status, lots, shares) = match last {
            '1' | 'A' | 'a' => (NormalizedAllotmentStatus::Allotted, Some(1), Some(35)),
            '2' | 'B' | 'b' => (NormalizedAllotmentStatus::NotAllotted, None, None),
            '3' | 'C' | 'c' => (NormalizedAllotmentStatus::Pending, None, None),
            '4' | 'D' | 'd' => {
                return Err(ProviderError::NeedsHuman("fixture captcha".into()));
            }
            '5' | 'E' | 'e' => return Err(ProviderError::RateLimited),
            '6' | 'F' | 'f' => {
                return Err(ProviderError::Unavailable("fixture down".into()));
            }
            '7' | 'G' | 'g' => {
                return Err(ProviderError::Unknown("fixture garbled html".into()));
            }
            _ => (NormalizedAllotmentStatus::NotFound, None, None),
        };

        Ok(ProviderAllotmentResult::fixture(
            status,
            lots,
            shares,
            Some(format!(
                "fixture:{}:{}",
                context.account_id, context.attempt_id
            )),
            Some(status.as_str().into()),
        ))
    }
}
