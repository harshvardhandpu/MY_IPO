//! Durable allotment job / attempt state machine. Account IDs only — never PAN.

use serde::{Deserialize, Serialize};

use crate::status::NormalizedAllotmentStatus;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AllotmentJobError {
    #[error("invalid job transition from {from} to {to}")]
    InvalidJobTransition { from: String, to: String },
    #[error("invalid attempt transition from {from} to {to}")]
    InvalidAttemptTransition { from: String, to: String },
    #[error("job field {0} cannot be empty")]
    EmptyField(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AllotmentJobStatus {
    Created,
    WaitingForProviderAvailability,
    PreparingProviderSession,
    Running,
    VerificationRequiredRefresh,
    PartiallyComplete,
    Complete,
}

impl AllotmentJobStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::WaitingForProviderAvailability => "WAITING_FOR_PROVIDER_AVAILABILITY",
            Self::PreparingProviderSession => "PREPARING_PROVIDER_SESSION",
            Self::Running => "RUNNING",
            Self::VerificationRequiredRefresh => "VERIFICATION_REQUIRED_REFRESH",
            Self::PartiallyComplete => "PARTIALLY_COMPLETE",
            Self::Complete => "COMPLETE",
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use AllotmentJobStatus::*;
        matches!(
            (self, next),
            (Created, WaitingForProviderAvailability)
                | (Created, PreparingProviderSession)
                | (Created, Running)
                | (WaitingForProviderAvailability, PreparingProviderSession)
                | (WaitingForProviderAvailability, Running)
                | (WaitingForProviderAvailability, Complete)
                | (PreparingProviderSession, Running)
                | (PreparingProviderSession, VerificationRequiredRefresh)
                | (VerificationRequiredRefresh, PreparingProviderSession)
                | (Running, PartiallyComplete)
                | (Running, VerificationRequiredRefresh)
                | (Running, Complete)
                | (PartiallyComplete, Running)
                | (PartiallyComplete, Complete)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AttemptStatus {
    Pending,
    PreparingProviderSession,
    Running,
    Allotted,
    NotAllotted,
    NotFound,
    Unknown,
    NeedsHumanVerification,
    VerificationRequiredRefresh,
    RateLimited,
    ProviderUnavailable,
    RetryableError,
    ManualResult,
}

impl AttemptStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::PreparingProviderSession => "PREPARING_PROVIDER_SESSION",
            Self::Running => "RUNNING",
            Self::Allotted => "ALLOTTED",
            Self::NotAllotted => "NOT_ALLOTTED",
            Self::NotFound => "NOT_FOUND",
            Self::Unknown => "UNKNOWN",
            Self::NeedsHumanVerification => "NEEDS_HUMAN_VERIFICATION",
            Self::VerificationRequiredRefresh => "VERIFICATION_REQUIRED_REFRESH",
            Self::RateLimited => "RATE_LIMITED",
            Self::ProviderUnavailable => "PROVIDER_UNAVAILABLE",
            Self::RetryableError => "RETRYABLE_ERROR",
            Self::ManualResult => "MANUAL_RESULT",
        }
    }

    pub fn from_normalized(s: NormalizedAllotmentStatus) -> Self {
        match s {
            NormalizedAllotmentStatus::Allotted => Self::Allotted,
            NormalizedAllotmentStatus::NotAllotted => Self::NotAllotted,
            NormalizedAllotmentStatus::Pending => Self::Pending,
            NormalizedAllotmentStatus::NotFound => Self::NotFound,
            NormalizedAllotmentStatus::Unknown => Self::Unknown,
            NormalizedAllotmentStatus::NeedsHumanVerification => Self::NeedsHumanVerification,
            NormalizedAllotmentStatus::RateLimited => Self::RateLimited,
            NormalizedAllotmentStatus::ProviderUnavailable => Self::ProviderUnavailable,
            NormalizedAllotmentStatus::RetryableError => Self::RetryableError,
            NormalizedAllotmentStatus::ManualResult => Self::ManualResult,
        }
    }

    pub const fn is_final(self) -> bool {
        matches!(
            self,
            Self::Allotted | Self::NotAllotted | Self::NotFound | Self::ManualResult
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AllotmentResultSource {
    Provider,
    Manual,
    Fixture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ManualReportedOutcome {
    Allotted,
    NotAllotted,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllotmentCheckJob {
    id: String,
    application_id: String,
    session_id: String,
    ipo_name: String,
    registrar_id: String,
    registrar_name: String,
    official_status_url: Option<String>,
    status: AllotmentJobStatus,
    provider_id: String,
}

impl AllotmentCheckJob {
    pub fn create(
        id: impl Into<String>,
        application_id: impl Into<String>,
        session_id: impl Into<String>,
        ipo_name: impl Into<String>,
        registrar_id: impl Into<String>,
        registrar_name: impl Into<String>,
        provider_id: impl Into<String>,
    ) -> Result<Self, AllotmentJobError> {
        let id = id.into();
        let application_id = application_id.into();
        let session_id = session_id.into();
        let ipo_name = ipo_name.into();
        let registrar_id = registrar_id.into();
        let registrar_name = registrar_name.into();
        let provider_id = provider_id.into();
        for (name, v) in [
            ("id", id.as_str()),
            ("application_id", application_id.as_str()),
            ("session_id", session_id.as_str()),
            ("ipo_name", ipo_name.as_str()),
            ("registrar_id", registrar_id.as_str()),
            ("provider_id", provider_id.as_str()),
        ] {
            if v.trim().is_empty() {
                return Err(AllotmentJobError::EmptyField(name));
            }
        }
        Ok(Self {
            id,
            application_id,
            session_id,
            ipo_name,
            registrar_id,
            registrar_name,
            official_status_url: None,
            status: AllotmentJobStatus::Created,
            provider_id,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn application_id(&self) -> &str {
        &self.application_id
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn ipo_name(&self) -> &str {
        &self.ipo_name
    }
    pub fn registrar_id(&self) -> &str {
        &self.registrar_id
    }
    pub fn registrar_name(&self) -> &str {
        &self.registrar_name
    }
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }
    pub fn status(&self) -> AllotmentJobStatus {
        self.status
    }
    pub fn official_status_url(&self) -> Option<&str> {
        self.official_status_url.as_deref()
    }

    pub fn set_official_status_url(&mut self, url: Option<String>) {
        self.official_status_url = url;
    }

    pub fn transition_to(&mut self, next: AllotmentJobStatus) -> Result<(), AllotmentJobError> {
        if !self.status.can_transition_to(next) {
            return Err(AllotmentJobError::InvalidJobTransition {
                from: self.status.as_str().into(),
                to: next.as_str().into(),
            });
        }
        self.status = next;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllotmentCheckAttempt {
    id: String,
    job_id: String,
    account_id: String,
    status: AttemptStatus,
    attempt_count: u32,
    allotted_lots: Option<u32>,
    allotted_shares: Option<u64>,
    provider_reference: Option<String>,
    safe_message: Option<String>,
    source: AllotmentResultSource,
    manual_reported_outcome: Option<ManualReportedOutcome>,
    last_attempt_at: Option<String>,
    next_retry_at: Option<String>,
}

impl AllotmentCheckAttempt {
    pub fn new(
        id: impl Into<String>,
        job_id: impl Into<String>,
        account_id: impl Into<String>,
    ) -> Result<Self, AllotmentJobError> {
        let id = id.into();
        let job_id = job_id.into();
        let account_id = account_id.into();
        if id.trim().is_empty() || job_id.trim().is_empty() || account_id.trim().is_empty() {
            return Err(AllotmentJobError::EmptyField("attempt ids"));
        }
        Ok(Self {
            id,
            job_id,
            account_id,
            status: AttemptStatus::Pending,
            attempt_count: 0,
            allotted_lots: None,
            allotted_shares: None,
            provider_reference: None,
            safe_message: None,
            source: AllotmentResultSource::Provider,
            manual_reported_outcome: None,
            last_attempt_at: None,
            next_retry_at: None,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    pub fn account_id(&self) -> &str {
        &self.account_id
    }
    pub fn status(&self) -> AttemptStatus {
        self.status
    }
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count
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
    pub fn safe_message(&self) -> Option<&str> {
        self.safe_message.as_deref()
    }
    pub fn source(&self) -> AllotmentResultSource {
        self.source
    }
    pub fn manual_reported_outcome(&self) -> Option<ManualReportedOutcome> {
        self.manual_reported_outcome
    }

    pub fn mark_running(&mut self, at: impl Into<String>) {
        self.status = AttemptStatus::Running;
        self.attempt_count = self.attempt_count.saturating_add(1);
        self.last_attempt_at = Some(at.into());
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_normalized(
        &mut self,
        status: NormalizedAllotmentStatus,
        lots: Option<u32>,
        shares: Option<u64>,
        provider_reference: Option<String>,
        safe_message: Option<String>,
        source: AllotmentResultSource,
        next_retry_at: Option<String>,
    ) {
        self.status = AttemptStatus::from_normalized(status);
        self.allotted_lots = lots;
        self.allotted_shares = shares;
        self.provider_reference = provider_reference;
        self.safe_message = safe_message;
        self.source = source;
        self.manual_reported_outcome = None;
        self.next_retry_at = if status.is_final() {
            None
        } else {
            next_retry_at
        };
    }

    pub fn apply_manual(&mut self, input: &ManualResultInput) {
        self.apply_normalized(
            NormalizedAllotmentStatus::ManualResult,
            input.allotted_lots,
            input.allotted_shares,
            None,
            input.note.clone(),
            AllotmentResultSource::Manual,
            None,
        );
        self.manual_reported_outcome = Some(
            if input.allotted_shares.unwrap_or(0) > 0 || input.allotted_lots.unwrap_or(0) > 0 {
                ManualReportedOutcome::Allotted
            } else if input.explicit_not_allotted {
                ManualReportedOutcome::NotAllotted
            } else {
                ManualReportedOutcome::Unknown
            },
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualResultInput {
    pub allotted_lots: Option<u32>,
    pub allotted_shares: Option<u64>,
    pub explicit_not_allotted: bool,
    pub note: Option<String>,
    pub actor_member_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_rejects_invalid_transition() {
        let mut job = AllotmentCheckJob::create(
            "j1",
            "a1",
            "s1",
            "IPO",
            "kfintech",
            "KFin",
            "kfintech-fixture",
        )
        .unwrap();
        assert!(job.transition_to(AllotmentJobStatus::Complete).is_err());
        job.transition_to(AllotmentJobStatus::Running).unwrap();
        job.transition_to(AllotmentJobStatus::Complete).unwrap();
    }

    #[test]
    fn attempt_final_clears_retry() {
        let mut a = AllotmentCheckAttempt::new("t1", "j1", "acct").unwrap();
        a.apply_normalized(
            NormalizedAllotmentStatus::Allotted,
            Some(1),
            Some(35),
            None,
            None,
            AllotmentResultSource::Fixture,
            Some("later".into()),
        );
        assert!(a.is_final_like());
        assert!(a.next_retry_at.is_none());
    }

    impl AllotmentCheckAttempt {
        fn is_final_like(&self) -> bool {
            self.status.is_final()
        }
    }
}
