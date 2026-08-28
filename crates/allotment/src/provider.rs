//! Provider-independent allotment contract. No CSS/HTML/browser details here.

use serde::{Deserialize, Serialize};

use sanket_identity_security::Pan;

use crate::status::NormalizedAllotmentStatus;

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

/// Safe provider outcome — never contains PAN.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAllotmentResult {
    pub status: NormalizedAllotmentStatus,
    pub allotted_lots: Option<u32>,
    pub allotted_shares: Option<u64>,
    pub provider_reference: Option<String>,
    pub checked_at: String,
    pub safe_message: Option<String>,
    pub evidence_ref: Option<String>,
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
    fn health(&self) -> ProviderHealth;
    fn supports(&self, issue: &RegistrarIssue) -> bool;

    /// PAN is a temporary argument. Implementations must not store it.
    fn check_allotment(
        &self,
        context: &AllotmentLookupContext,
        pan: &Pan,
    ) -> Result<ProviderAllotmentResult, ProviderError>;
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

        Ok(ProviderAllotmentResult {
            status,
            allotted_lots: lots,
            allotted_shares: shares,
            provider_reference: Some(format!(
                "fixture:{}:{}",
                context.account_id, context.attempt_id
            )),
            checked_at: "fixture".into(),
            safe_message: Some(status.as_str().into()),
            evidence_ref: None,
        })
    }
}
